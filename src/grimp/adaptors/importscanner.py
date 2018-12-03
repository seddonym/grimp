from typing import Set, List
import ast

from grimp.application.ports.importscanner import AbstractImportScanner
from grimp.domain.valueobjects import Module, DirectImport


class ImportScanner(AbstractImportScanner):
    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        """
        Note: this method only analyses the module in question and will not load any other
        code, so it relies on self.modules to deduce which modules it imports. (This is
        because you can't know whether "from foo.bar import baz" is importing a module
        called  `baz`, or a function `baz` from the module `bar`.)
        """
        direct_imports: Set[DirectImport] = set()

        module_contents = self._read_module_contents(module)
        module_lines = module_contents.splitlines()
        ast_tree = ast.parse(module_contents)
        for node in ast.walk(ast_tree):
            d = self._parse_direct_imports_from_node(node, module, module_lines)
            direct_imports |= d

        # imported_modules = self._trim_each_to_known_modules(imported_modules)
        return direct_imports

    def _parse_direct_imports_from_node(
            self, node: ast.AST, module: Module, module_lines: List[str],
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
                module_base = '.'.join(importing_module_components[:-node.level])
                if node.module:
                    module_base = '.'.join([module_base, node.module])

            # node.names corresponds to 'a', 'b' and 'c' in 'from x import a, b, c'.
            for alias in node.names:
                full_module_name = '.'.join([module_base, alias.name])
                direct_imports.add(
                    self._build_direct_import(
                        importer=module,
                        untrimmed_imported=Module(full_module_name),
                        node=node,
                        module_lines=module_lines,
                    ),
                )

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
        imported = self._trim_to_known_module(untrimmed_imported)
        return DirectImport(
            importer=importer,
            imported=imported,
            line_number=node.lineno,
            line_contents=module_lines[node.lineno - 1].strip(),
        )

    def _trim_to_known_module(self, untrimmed_module: Module) -> Module:
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
                # TODO - should we handle this more gracefully?
                raise ValueError(f'Could not trim {untrimmed_module}.')

    def _read_module_contents(self, module: Module) -> str:
        """
        Read the file contents of the module.

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
            try:
                return self.file_system.read(candidate_filename)
            except FileNotFoundError:
                pass
        raise FileNotFoundError(f'Could not find module {module}.')
