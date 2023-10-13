from typing import Dict, Optional, Set

from grimp.application.ports.modulefinder import FoundPackage, ModuleFile
from grimp.domain.valueobjects import DirectImport, Module

from .filesystem import AbstractFileSystem


class CacheMiss(Exception):
    pass


class Cache:
    def __init__(
        self,
        file_system: AbstractFileSystem,
        include_external_packages: bool,
        exclude_type_checking_imports: bool,
        found_packages: Set[FoundPackage],
        cache_dir: str,
    ) -> None:
        """
        Don't instantiate Cache directly; use Cache.setup().
        """
        self.file_system = file_system
        self.found_packages = found_packages
        self.include_external_packages = include_external_packages
        self.exclude_type_checking_imports = exclude_type_checking_imports
        self.cache_dir = cache_dir

    @classmethod
    def setup(
        cls,
        file_system: AbstractFileSystem,
        found_packages: Set[FoundPackage],
        *,
        include_external_packages: bool,
        exclude_type_checking_imports: bool = False,
        cache_dir: Optional[str] = None,
    ) -> "Cache":
        cache = cls(
            file_system=file_system,
            found_packages=found_packages,
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
            cache_dir=cls.cache_dir_or_default(cache_dir),
        )
        return cache

    def read_imports(self, module_file: ModuleFile) -> Set[DirectImport]:
        raise NotImplementedError

    def write(
        self,
        imports_by_module: Dict[Module, Set[DirectImport]],
    ) -> None:
        raise NotImplementedError

    @classmethod
    def cache_dir_or_default(cls, cache_dir: Optional[str]) -> str:
        raise NotImplementedError
