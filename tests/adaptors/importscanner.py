from typing import Dict, Set

from grimp.application.ports.importscanner import AbstractImportScanner
from grimp.domain.valueobjects import DirectImport, Module


class BaseFakeImportScanner(AbstractImportScanner):
    """
    Usage:

        To emulate a module foo.one that imports foo.two and foo.three, do:

            class FakeImportScanner(BaseFakeImportScanner):
                import_map = {
                    Module('foo.one'): (Module('foo.two'), Module('foo.three')),
                }

    import_map: map of the imports within each module. Keys are each module, values are
                modules that the module directly imports.
    """
    import_map: Dict[Module, Set[Module]] = {}

    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        try:
            imported_modules = self.import_map[module]
        except KeyError:
            return set()

        build_direct_import = lambda imported_module: DirectImport(
            importer=module,
            imported=imported_module
        )

        return map(build_direct_import, imported_modules)
