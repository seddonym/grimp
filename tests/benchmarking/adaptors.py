from grimp.adaptors.caching import Cache, CacheMiss
from typing import Set

from grimp.application.ports.modulefinder import ModuleFile
from grimp.domain.valueobjects import DirectImport


class PrefixMissingCache(Cache):
    """
    Test double of the real cache that will miss caching any module that begins with
    a special prefix.
    """

    MISSING_PREFIX = "miss_marker_8772f06d64b6_"  # Arbitrary prefix.

    def read_imports(self, module_file: ModuleFile) -> Set[DirectImport]:
        leaf_name = module_file.module.name.split(".")[-1]
        if leaf_name.startswith(self.MISSING_PREFIX):
            raise CacheMiss
        return super().read_imports(module_file)
