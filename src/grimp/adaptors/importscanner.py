from __future__ import annotations

import ast
import re
import logging
from dataclasses import dataclass
from typing import Dict, List, Optional, Set, Union
from ast import NodeVisitor, Import, ImportFrom, If, Attribute, Name

from grimp import exceptions
from grimp.application.ports.importscanner import AbstractImportScanner
from grimp.application.ports.modulefinder import FoundPackage
from grimp.domain.valueobjects import DirectImport, Module
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

logger = logging.getLogger(__name__)

_LEADING_DOT_REGEX = re.compile(r"^(\.+)\w")


@dataclass(frozen=True)
class _ImportedObject:
    name: str
    line_number: int
    line_contents: str
    typechecking_only: bool


class ImportScanner(AbstractImportScanner):
    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)

        self._found_packages_by_module: Dict[Module, FoundPackage] = {
            module_file.module: package
            for package in self.found_packages
            for module_file in package.module_files
        }

    def scan_for_imports(
        self, module: Module, *, exclude_type_checking_imports: bool = False
    ) -> Set[DirectImport]:
        """
        Note: this method only analyses the module in question and will not load any other
        code, so it relies on self.modules to deduce which modules it imports. (This is
        because you can't know whether "from foo.bar import baz" is importing a module
        called  `baz`, or a function `baz` from the module `bar`.)
        """
        found_package = self._found_package_for_module(module)
        module_filename = self._determine_module_filename(module, found_package)
        module_contents = self._read_module_contents(module_filename)

        try:
            imported_objects = self._get_raw_imported_objects(module_contents)
        except SyntaxError as e:
            raise exceptions.SourceSyntaxError(
                filename=module_filename,
                lineno=e.lineno,
                text=e.text,
            )

        is_package = self._module_is_package(module_filename)

        imports = set()
        for imported_object in imported_objects:
            # Filter on `exclude_type_checking_imports`.
            if exclude_type_checking_imports and imported_object.typechecking_only:
                continue

            # Resolve relative imports.
            imported_object_name = self._get_absolute_imported_object_name(
                module=module, is_package=is_package, imported_object_name=imported_object.name
            )

            # Resolve imported module.
            imported_module = self._get_internal_module(imported_object_name, modules=self.modules)
            if imported_module is None:
                # => External import.

                # Filter on `self.include_external_packages`.
                if not self.include_external_packages:
                    continue

                # Distill module.
                imported_module = self._distill_external_module(
                    Module(imported_object_name), found_packages=self.found_packages
                )
                if imported_module is None:
                    continue

            imports.add(
                DirectImport(
                    importer=module,
                    imported=imported_module,
                    line_number=imported_object.line_number,
                    line_contents=imported_object.line_contents,
                )
            )
        return imports

    def _found_package_for_module(self, module: Module) -> FoundPackage:
        try:
            return self._found_packages_by_module[module]
        except KeyError:
            raise ValueError(f"No found package for module {module}.")

    def _determine_module_filename(self, module: Module, found_package: FoundPackage) -> str:
        """
        Work out the full filename of the given module.

        Any given module can either be a straight Python file (foo.py) or else a package
        (in which case the file is an __init__.py within a directory).
        """
        top_level_components = found_package.name.split(".")
        module_components = module.name.split(".")
        leaf_components = module_components[len(top_level_components) :]
        package_directory = found_package.directory

        filename_root = self.file_system.join(package_directory, *leaf_components)
        candidate_filenames = (
            f"{filename_root}.py",
            self.file_system.join(filename_root, "__init__.py"),
        )
        for candidate_filename in candidate_filenames:
            if self.file_system.exists(candidate_filename):
                return candidate_filename
        raise FileNotFoundError(f"Could not find module {module}.")

    def _read_module_contents(self, module_filename: str) -> str:
        """
        Read the file contents of the module.
        """
        return self.file_system.read(module_filename)

    def _module_is_package(self, module_filename: str) -> bool:
        """
        Whether or not the supplied module filename is a package.
        """
        return self.file_system.split(module_filename)[-1] == "__init__.py"

    @staticmethod
    def _get_raw_imported_objects(module_contents: str) -> Set[_ImportedObject]:
        dicts_from_rust = rust.parse_to_imported_objects(module_contents)
        objects_from_rust = {_ImportedObject(**object_kwargs) for object_kwargs in dicts_from_rust}

        # TODO - remove these lines once we're confident the rust way
        # is consistent with ast.
        #
        # module_lines = module_contents.splitlines()
        # ast_tree = ast.parse(module_contents)
        # visitor = _TreeVisitor(module_lines=module_lines)
        # visitor.visit(ast_tree)
        # objects_from_ast = visitor.imported_objects
        #
        # assert objects_from_rust == objects_from_ast, "Discrepancy!"

        return objects_from_rust

    @staticmethod
    def _get_absolute_imported_object_name(
        *, module: Module, is_package: bool, imported_object_name: str
    ) -> str:
        leading_dot_match = _LEADING_DOT_REGEX.match(imported_object_name)
        if leading_dot_match is None:
            return imported_object_name

        n_leading_dots = len(leading_dot_match.group(1))
        if is_package:
            if n_leading_dots == 1:
                imported_object_name_base = module.name
            else:
                imported_object_name_base = ".".join(
                    module.name.split(".")[: -(n_leading_dots - 1)]
                )
        else:
            imported_object_name_base = ".".join(module.name.split(".")[:-n_leading_dots])
        return imported_object_name_base + "." + imported_object_name[n_leading_dots:]

    @staticmethod
    def _get_internal_module(object_name: str, *, modules: Set[Module]) -> Optional[Module]:
        candidate_module = Module(object_name)
        if candidate_module in modules:
            return candidate_module

        try:
            candidate_module = candidate_module.parent
        except ValueError:
            return None
        else:
            if candidate_module in modules:
                return candidate_module
            else:
                return None

    @staticmethod
    def _distill_external_module(
        module: Module, *, found_packages: Set[FoundPackage]
    ) -> Optional[Module]:
        """
        Given a module that we already know is external, turn it into a module to add to the graph.

        The 'distillation' process involves removing any unwanted subpackages. For example,
        Module("django.models.db") should be turned into simply Module("django").

        The process is more complex for potential namespace packages, as it's not possible to
        determine the portion package simply from name. Rather than adding the overhead of a
        filesystem read, we just get the shallowest component that does not clash with an internal
        module namespace. Take, for example, a Module("foo.blue.alpha.one"). If one of the found
        packages is foo.blue.beta, the module will be distilled to Module("foo.blue.alpha").
        Alternatively, if the found package is foo.green, the distilled module will
        be Module("foo.blue").
        """
        # If it's a module that is a parent of one of the internal packages, return None
        # as it doesn't make sense and is probably an import of a namespace package.
        if any(Module(package.name).is_descendant_of(module) for package in found_packages):
            return None

        # If it shares a namespace with an internal module, get the shallowest component that does
        # not clash with an internal module namespace.
        candidate_portions: Set[Module] = set()
        for found_package in sorted(found_packages, key=lambda p: p.name, reverse=True):
            root_module = Module(found_package.name)
            if root_module.is_descendant_of(module.root):
                (
                    internal_path_components,
                    external_path_components,
                ) = root_module.name.split(
                    "."
                ), module.name.split(".")
                external_namespace_components = []
                while external_path_components[0] == internal_path_components[0]:
                    external_namespace_components.append(external_path_components[0])
                    external_path_components = external_path_components[1:]
                    internal_path_components = internal_path_components[1:]
                external_namespace_components.append(external_path_components[0])
                candidate_portions.add(Module(".".join(external_namespace_components)))

        if candidate_portions:
            # If multiple found packages share a namespace with this module, use the deepest one
            # as we know that that will be a namespace too.
            deepest_candidate_portion = sorted(
                candidate_portions, key=lambda p: len(p.name.split("."))
            )[-1]
            return deepest_candidate_portion
        else:
            return module.root


class _TreeVisitor(NodeVisitor):
    def __init__(
        self,
        module_lines: List[str],
    ) -> None:
        self.import_parser = _ImportNodeParser()
        self.from_import_parser = _ImportFromNodeParser()
        self.module_lines = module_lines

        self.imported_objects: Set[_ImportedObject] = set()
        self.typechecking_only = False

        super().__init__()

    def visit_Import(self, node: Import) -> None:
        self._parse_imported_objects_from_node(node, self.import_parser)

    def visit_ImportFrom(self, node: ImportFrom) -> None:
        self._parse_imported_objects_from_node(node, self.from_import_parser)

    def visit_If(self, node: If) -> None:
        if (isinstance(node.test, Name) and node.test.id == "TYPE_CHECKING") or (
            isinstance(node.test, Attribute) and node.test.attr == "TYPE_CHECKING"
        ):
            self.typechecking_only = True
            super().generic_visit(node)
            self.typechecking_only = False
        else:
            super().generic_visit(node)

    def _parse_imported_objects_from_node(
        self,
        node: Union[Import, ImportFrom],
        parser: Union[_ImportNodeParser, _ImportFromNodeParser],
    ) -> None:
        for imported_object in parser.determine_imported_objects(node):
            self.imported_objects.add(
                _ImportedObject(
                    name=imported_object,
                    line_number=node.lineno,
                    line_contents=self.module_lines[node.lineno - 1].strip(),
                    typechecking_only=self.typechecking_only,
                )
            )


class _ImportNodeParser:
    """
    Parser for statements in the form 'import x'.
    """

    node_class = ast.Import

    def determine_imported_objects(self, node: ast.AST) -> Set[str]:
        imported_objects: Set[str] = set()
        assert isinstance(node, self.node_class)  # For type checker.
        for alias in node.names:
            imported_object = alias.name
            imported_objects.add(imported_object)
        return imported_objects


class _ImportFromNodeParser:
    """
    Parser for statements in the form 'from x import ...'.
    """

    node_class = ast.ImportFrom

    def determine_imported_objects(self, node: ast.AST) -> Set[str]:
        imported_objects: Set[str] = set()
        assert isinstance(node, self.node_class)  # For type checker.
        assert isinstance(node.level, int)  # For type checker.

        for alias in node.names:
            if node.module is None:
                imported_object = f"{'.' * node.level}{alias.name}"
            else:
                imported_object = f"{'.' * node.level}{node.module}.{alias.name}"
            imported_objects.add(imported_object)

        return imported_objects
