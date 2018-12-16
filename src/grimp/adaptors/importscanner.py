from typing import Set, List
import ast

import logging

from grimp.application.ports.importscanner import AbstractImportScanner
from grimp.domain.valueobjects import Module, DirectImport


logger = logging.getLogger(__name__)


class ImportScanner(AbstractImportScanner):
    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        """
        Note: this method only analyses the module in question and will not load any other
        code, so it relies on self.modules to deduce which modules it imports. (This is
        because you can't know whether "from foo.bar import baz" is importing a module
        called  `baz`, or a function `baz` from the module `bar`.)
        """
        direct_imports: Set[DirectImport] = set()

        module_filename = self._determine_module_filename(module)
        is_package = self._module_is_package(module_filename)
        module_contents = self._read_module_contents(module_filename)
        module_lines = module_contents.splitlines()
        ast_tree = ast.parse(module_contents)
        for node in ast.walk(ast_tree):
            d = self._parse_direct_imports_from_node(node, module, module_lines, is_package)
            direct_imports |= d

        # imported_modules = self._trim_each_to_known_modules(imported_modules)
        return direct_imports

    def _parse_direct_imports_from_node(
            self, node: ast.AST, module: Module, module_lines: List[str], is_package: bool,
    ) -> Set[DirectImport]:
        """
        Parse an ast node into a set of DirectImports.
        """
        direct_imports: Set[DirectImport] = set()

        if isinstance(node, ast.ImportFrom):
            # Parsing something in the form 'from x import ...'.
            assert isinstance(node.level, int)
            if node.level == 0:
                # Absolute import.
                # Let the type checker know we expect node.module to be set here.
                assert isinstance(node.module, str)
                if not node.module.startswith(module.package_name):
                    # Don't include imports of modules outside this package.
                    return set()
                module_base = node.module
            elif node.level >= 1:
                # Relative import. The level corresponds to how high up the tree it goes;
                # for example 'from ... import foo' would be level 3.
                importing_module_components = module.name.split('.')
                # TODO: handle level that is too high.
                # Trim the base module by the number of levels.
                if is_package:
                    # If the scanned module an __init__.py file, we don't want
                    # to go up an extra level.
                    number_of_levels_to_trim_by = node.level - 1
                else:
                    number_of_levels_to_trim_by = node.level

                if number_of_levels_to_trim_by:
                    module_base = '.'.join(
                        importing_module_components[:-number_of_levels_to_trim_by]
                    )
                else:
                    module_base = '.'.join(importing_module_components)
                if node.module:
                    module_base = '.'.join([module_base, node.module])

            # node.names corresponds to 'a', 'b' and 'c' in 'from x import a, b, c'.
            for alias in node.names:
                full_module_name = '.'.join([module_base, alias.name])
                try:
                    direct_import = self._build_direct_import(
                        importer=module,
                        untrimmed_imported=Module(full_module_name),
                        node=node,
                        module_lines=module_lines,
                    )
                except FileNotFoundError:
                    logger.warning(
                        f'Could not find {full_module_name} when scanning {module}. '
                        'This may be due to a missing __init__.py file in the parent package.'
                    )
                else:
                    direct_imports.add(direct_import)

        elif isinstance(node, ast.Import):
            # Parsing a line in the form 'import x'.
            for alias in node.names:
                if not alias.name.startswith(module.package_name):
                    continue
                direct_imports.add(
                    DirectImport(
                        importer=module,
                        imported=Module(alias.name),
                        line_number=node.lineno,
                        line_contents=module_lines[node.lineno - 1],
                    )
                )

        return direct_imports

    def _build_direct_import(self, importer: Module, untrimmed_imported: Module,
                             node: ast.AST, module_lines: List[str]) -> DirectImport:
        """
        Raises FileNotFoundError if it could not find a valid module.
        """
        imported = self._trim_to_known_module(untrimmed_imported)
        return DirectImport(
            importer=importer,
            imported=imported,
            line_number=node.lineno,
            line_contents=module_lines[node.lineno - 1].strip(),
        )

    def _trim_to_known_module(self, untrimmed_module: Module) -> Module:
        """
        Raises FileNotFoundError if it could not find a valid module.
        """
        if untrimmed_module in self.modules:
            return untrimmed_module
        else:
            # The module isn't in the known modules. This is because it's something *within*
            # a module (e.g. a function): the result of something like 'from .subpackage
            # import my_function'. So we trim the components back to the module.
            components = untrimmed_module.name.split('.')[:-1]
            trimmed_module = Module('.'.join(components))

            if trimmed_module in self.modules:
                return trimmed_module
            else:
                raise FileNotFoundError()

    def _determine_module_filename(self, module: Module) -> str:
        """
        Work out the full filename of the given module.

        Any given module can either be a straight Python file (foo.py) or else a package
        (in which case the file is an __init__.py within a directory).
        """
        module_components = module.name.split('.')
        package_directory_parts = self.file_system.split(self.package_directory)
        assert module_components[0] == package_directory_parts[-1], (
            'The package directory should be the same as the first part of the module name.')

        filename_root = self.file_system.join(self.package_directory, *module_components[1:])
        candidate_filenames = (
            f'{filename_root}.py',
            self.file_system.join(filename_root, '__init__.py'),
        )
        for candidate_filename in candidate_filenames:
            if self.file_system.exists(candidate_filename):
                return candidate_filename
        raise FileNotFoundError(f'Could not find module {module}.')

    def _read_module_contents(self, module_filename: str) -> str:
        """
        Read the file contents of the module.
        """
        return self.file_system.read(module_filename)

    def _module_is_package(self, module_filename: str) -> bool:
        """
        Whether or not the supplied module filename is a package.
        """
        return self.file_system.split(module_filename)[-1] == '__init__.py'
