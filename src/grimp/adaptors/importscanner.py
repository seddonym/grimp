from typing import Set
import ast

from grimp.application.ports.filesystem import AbstractFileSystem
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
        imported_modules = set()

        module_contents = self._read_module_contents(module)

        ast_tree = ast.parse(module_contents)
        for node in ast.walk(ast_tree):
            if isinstance(node, ast.ImportFrom):
                # Parsing something in the form 'from x import ...'.
                assert isinstance(node.level, int)
                if node.level == 0:
                    # Absolute import.
                    # Let the type checker know we expect node.module to be set here.
                    assert isinstance(node.module, str)
                    if not node.module.startswith(module.package_name):
                        # Don't include imports of modules outside this package.
                        continue
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
                    imported_modules.add(Module(full_module_name))

            elif isinstance(node, ast.Import):
                # Parsing a line in the form 'import x'.
                for alias in node.names:
                    if not alias.name.startswith(module.package_name):
                        # Don't include imports of modules outside this package.
                        continue
                    imported_modules.add(Module(alias.name))
            else:
                # Not an import statement; move on.
                continue

        imported_modules = self._trim_each_to_known_modules(imported_modules)
        return {DirectImport(importer=module, imported=imported) for imported in imported_modules}

    def _trim_each_to_known_modules(self, imported_modules: Set[Module]) -> Set[Module]:
        known_modules = set()
        for imported_module in imported_modules:
            if imported_module in self.modules:
                known_modules.add(imported_module)
            else:
                # The module isn't in the known modules. This is because it's something *within*
                # a module (e.g. a function): the result of something like 'from .subpackage
                # import my_function'. So we trim the components back to the module.
                components = imported_module.name.split('.')[:-1]
                trimmed_module = Module('.'.join(components))
                if trimmed_module in self.modules:
                    known_modules.add(trimmed_module)
                else:
                    # TODO: we may want to warn about this.
                    # logger.debug('{} not found in modules.'.format(trimmed_module))
                    pass
        return known_modules

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
