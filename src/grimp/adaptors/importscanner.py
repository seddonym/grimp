import ast
import logging
from typing import List, Optional, Set

from grimp import exceptions
from grimp.application.ports.importscanner import AbstractImportScanner
from grimp.application.ports.modulefinder import FoundPackage
from grimp.domain.valueobjects import DirectImport, Module

logger = logging.getLogger(__name__)


class NotAnImport(Exception):
    pass


class ImportScanner(AbstractImportScanner):
    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        """
        Note: this method only analyses the module in question and will not load any other
        code, so it relies on self.modules to deduce which modules it imports. (This is
        because you can't know whether "from foo.bar import baz" is importing a module
        called  `baz`, or a function `baz` from the module `bar`.)
        """
        direct_imports: Set[DirectImport] = set()

        found_package = self._found_package_for_module(module)
        module_filename = self._determine_module_filename(module, found_package)
        is_package = self._module_is_package(module_filename)
        module_contents = self._read_module_contents(module_filename)
        module_lines = module_contents.splitlines()
        try:
            ast_tree = ast.parse(module_contents)
        except SyntaxError as e:
            raise exceptions.SourceSyntaxError(
                filename=module_filename,
                lineno=e.lineno,
                text=e.text,
            )
        for node in ast.walk(ast_tree):
            direct_imports |= self._parse_direct_imports_from_node(
                node, module, found_package, module_lines, is_package
            )

        return direct_imports

    def _parse_direct_imports_from_node(
        self,
        node: ast.AST,
        module: Module,
        found_package: FoundPackage,
        module_lines: List[str],
        is_package: bool,
    ) -> Set[DirectImport]:
        """
        Parse an ast node into a set of DirectImports.
        """
        try:
            parser = _get_node_parser(
                node=node,
                module=module,
                found_package=found_package,
                found_packages=self.found_packages,
                is_package=is_package,
            )
        except NotAnImport:
            return set()

        direct_imports: Set[DirectImport] = set()

        for imported in parser.determine_imported_modules(
            include_external_packages=self.include_external_packages
        ):
            direct_imports.add(
                DirectImport(
                    importer=module,
                    imported=imported,
                    line_number=node.lineno,
                    line_contents=module_lines[node.lineno - 1].strip(),
                )
            )

        return direct_imports

    def _determine_module_filename(
        self, module: Module, found_package: FoundPackage
    ) -> str:
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

    def _found_package_for_module(self, module: Module) -> FoundPackage:
        for package in self.found_packages:
            if module in package.modules:
                return package
        raise ValueError(f"No found package for module {module}.")

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


class _BaseNodeParser:
    """
    Works out from an AST node what the imported modules are.
    """

    def __init__(
        self,
        node: ast.AST,
        module: Module,
        found_package: FoundPackage,
        found_packages: Set[FoundPackage],
        is_package: bool,
    ) -> None:
        self.node = node
        self.module = module
        self.found_package = found_package
        self.found_packages = found_packages
        self.module_is_package = is_package

    def determine_imported_modules(
        self, include_external_packages: bool
    ) -> Set[Module]:
        """
        Return the imported modules in the statement.
        """
        raise NotImplementedError

    def _is_internal_module(self, module: Module) -> bool:
        return any(
            module in found_package.modules for found_package in self.found_packages
        )

    def _is_internal_object(self, full_object_name: str) -> bool:
        # Build a Module that may or may not exist.
        candidate_module = Module(full_object_name)
        if self._is_internal_module(candidate_module):
            return True

        # Also check the parent. In the case of non-module objects, this may be an internal module.
        try:
            parent = candidate_module.parent
        except ValueError:
            return False
        else:
            return self._is_internal_module(parent)

    def _distill_external_module(self, module: Module) -> Optional[Module]:
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
        if any(
            Module(package.name).is_descendant_of(module)
            for package in self.found_packages
        ):
            return None

        # If it shares a namespace with an internal module, get the shallowest component that does
        # not clash with an internal module namespace.
        candidate_portions: Set[Module] = set()
        for found_package in sorted(
            self.found_packages, key=lambda p: p.name, reverse=True
        ):
            root_module = Module(found_package.name)
            if root_module.is_descendant_of(module.root):
                (
                    internal_path_components,
                    external_path_components,
                ) = root_module.name.split("."), module.name.split(".")
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


class _ImportNodeParser(_BaseNodeParser):
    """
    Parser for statements in the form 'import x'.
    """

    node_class = ast.Import

    def determine_imported_modules(
        self, include_external_packages: bool
    ) -> Set[Module]:
        imported_modules: Set[Module] = set()

        assert isinstance(self.node, self.node_class)  # For type checker.
        for alias in self.node.names:
            imported_module = self._module_from_name(
                alias.name, include_external_packages=include_external_packages
            )
            if imported_module:
                imported_modules.add(imported_module)

        return imported_modules

    def _module_from_name(
        self, module_name: str, include_external_packages: bool
    ) -> Optional[Module]:
        module = Module(module_name)
        if self._is_internal_module(module):
            return module
        else:
            if include_external_packages:
                return self._distill_external_module(module)
            else:
                return None


class _ImportFromNodeParser(_BaseNodeParser):
    """
    Parser for statements in the form 'from x import ...'.
    """

    node_class = ast.ImportFrom

    def determine_imported_modules(
        self, include_external_packages: bool
    ) -> Set[Module]:
        imported_modules: Set[Module] = set()
        assert isinstance(self.node, self.node_class)  # For type checker.
        assert isinstance(self.node.level, int)  # For type checker.

        if self.node.level == 0:
            # Absolute import.
            # Let the type checker know we expect node.module to be set here.
            assert isinstance(self.node.module, str)
            node_module = Module(self.node.module)
            if not self._is_internal_module(node_module):
                if include_external_packages:
                    # Return the top level package of the external module.
                    external_modules = set()
                    for alias in self.node.names:
                        full_object_name = ".".join([self.node.module, alias.name])
                        untrimmed_module = Module(full_object_name)
                        external_module = self._distill_external_module(
                            untrimmed_module
                        )
                        if external_module:
                            external_modules.add(external_module)
                    return external_modules
                else:
                    return set()
            # Don't include imports of modules outside this package.

            module_base = self.node.module
        elif self.node.level >= 1:
            # Relative import. The level corresponds to how high up the tree it goes;
            # for example 'from ... import foo' would be level 3.
            importing_module_components = self.module.name.split(".")
            # TODO: handle level that is too high.
            # Trim the base module by the number of levels.
            if self.module_is_package:
                # If the scanned module an __init__.py file, we don't want
                # to go up an extra level.
                number_of_levels_to_trim_by = self.node.level - 1
            else:
                number_of_levels_to_trim_by = self.node.level

            if number_of_levels_to_trim_by:
                module_base = ".".join(
                    importing_module_components[:-number_of_levels_to_trim_by]
                )
            else:
                module_base = ".".join(importing_module_components)
            if self.node.module:
                module_base = ".".join([module_base, self.node.module])

        # node.names corresponds to 'a', 'b' and 'c' in 'from x import a, b, c'.
        for alias in self.node.names:
            full_object_name = ".".join([module_base, alias.name])
            imported_module = self._module_from_object_name(
                full_object_name, include_external_packages=include_external_packages
            )
            if imported_module:
                imported_modules.add(imported_module)

        return imported_modules

    def _trim_to_internal_module(self, untrimmed_module: Module) -> Module:
        """
        Raises FileNotFoundError if it could not find a valid module.
        """
        if self._is_internal_module(untrimmed_module):
            return untrimmed_module
        else:
            # The module isn't in the internal modules. This is because it's something *within*
            # a module (e.g. a function): the result of something like 'from .subpackage
            # import my_function'. So we trim the components back to the module.
            components = untrimmed_module.name.split(".")[:-1]
            trimmed_module = Module(".".join(components))

            if self._is_internal_module(trimmed_module):
                return trimmed_module
            else:
                raise FileNotFoundError()

    def _module_from_object_name(
        self, full_object_name: str, include_external_packages: bool
    ) -> Optional[Module]:
        if self._is_internal_object(full_object_name):
            untrimmed_module = Module(full_object_name)
            try:
                imported_module = self._trim_to_internal_module(
                    untrimmed_module=untrimmed_module
                )
            except FileNotFoundError:
                logger.warning(
                    f"Could not find {full_object_name} when scanning {self.module}. "
                    "This may be due to a missing __init__.py file in the parent package."
                )
            else:
                return imported_module
        else:
            untrimmed_module = Module(full_object_name)
            if include_external_packages:
                return self._distill_external_module(untrimmed_module)
        return None


def _get_node_parser(
    node: ast.AST,
    module: Module,
    found_package: FoundPackage,
    found_packages: Set[FoundPackage],
    is_package: bool,
) -> _BaseNodeParser:
    """
    Return a NodeParser instance for the supplied node.

    Raises NotAnImport if the supplied node is not an import statement.
    """
    parser_class_map = {
        ast.ImportFrom: _ImportFromNodeParser,
        ast.Import: _ImportNodeParser,
    }
    for node_class, parser_class in parser_class_map.items():
        if isinstance(node, node_class):
            return parser_class(
                node=node,
                module=module,
                found_package=found_package,
                found_packages=found_packages,
                is_package=is_package,
            )
    raise NotAnImport
