import hashlib
import json
import logging
from typing import Dict, List, Optional, Set, Tuple, Type

from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.application.ports.modulefinder import FoundPackage, ModuleFile
from grimp.domain.valueobjects import DirectImport, Module

from ..application.ports.caching import Cache as AbstractCache
from ..application.ports.caching import CacheMiss

logger = logging.getLogger(__name__)
PrimitiveFormat = Dict[str, List[Tuple[str, Optional[int], str]]]


class CacheFileNamer:
    @classmethod
    def make_meta_file_name(cls, found_package: FoundPackage) -> str:
        return f"{found_package.name}.meta.json"

    @classmethod
    def make_data_file_name(
        cls,
        found_packages: Set[FoundPackage],
        include_external_packages: bool,
        exclude_type_checking_imports: bool,
    ) -> str:
        identifier = cls.make_data_file_unique_string(
            found_packages, include_external_packages, exclude_type_checking_imports
        )

        bytes_identifier = identifier.encode()
        # Use a hash algorithm with a limited size to avoid cache filenames that are too long
        # the filesystem, which can happen if there are more than a few root packages
        # being analyzed.
        safe_unicode_identifier = hashlib.blake2b(bytes_identifier, digest_size=20).hexdigest()
        return f"{safe_unicode_identifier}.data.json"

    @classmethod
    def make_data_file_unique_string(
        cls,
        found_packages: Set[FoundPackage],
        include_external_packages: bool,
        exclude_type_checking_imports: bool,
    ) -> str:
        """
        Construct a unique string that identifies the analysis parameters.

        Doesn't need to be safe to use for a filename.
        """
        package_names = (p.name for p in found_packages)
        csv_packages = ",".join(sorted(package_names))
        include_external_packages_option = ":external" if include_external_packages else ""
        exclude_type_checking_imports_option = (
            ":no_type_checking" if exclude_type_checking_imports else ""
        )
        return (
            csv_packages + include_external_packages_option + exclude_type_checking_imports_option
        )


class Cache(AbstractCache):
    DEFAULT_CACHE_DIR = ".grimp_cache"

    def __init__(self, *args, namer: Type[CacheFileNamer], **kwargs) -> None:
        """
        Don't instantiate Cache directly; use Cache.setup().
        """
        super().__init__(*args, **kwargs)
        self._mtime_map: Dict[str, float] = {}
        self._data_map: Dict[Module, Set[DirectImport]] = {}
        self._namer = namer

    @classmethod
    def setup(
        cls,
        file_system: AbstractFileSystem,
        found_packages: Set[FoundPackage],
        include_external_packages: bool,
        exclude_type_checking_imports: bool = False,
        cache_dir: Optional[str] = None,
        namer: Type[CacheFileNamer] = CacheFileNamer,
    ) -> "Cache":
        cache = cls(
            file_system=file_system,
            found_packages=found_packages,
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
            cache_dir=cls.cache_dir_or_default(cache_dir),
            namer=namer,
        )
        cache._build_mtime_map()
        cache._build_data_map()
        assert cache.cache_dir
        return cache

    @classmethod
    def cache_dir_or_default(cls, cache_dir: Optional[str]) -> str:
        return cache_dir or cls.DEFAULT_CACHE_DIR

    def read_imports(self, module_file: ModuleFile) -> Set[DirectImport]:
        try:
            cached_mtime = self._mtime_map[module_file.module.name]
        except KeyError:
            raise CacheMiss
        if cached_mtime != module_file.mtime:
            raise CacheMiss

        try:
            return self._data_map[module_file.module]
        except KeyError:
            # While we would expect the module to be in here,
            # there's no point in crashing if, for some reason, it's not.
            raise CacheMiss

    def write(
        self,
        imports_by_module: Dict[Module, Set[DirectImport]],
    ) -> None:
        self._write_marker_files_if_not_already_there()
        # Write data file.
        primitives_map: PrimitiveFormat = {}
        for found_package in self.found_packages:
            primitives_map_for_found_package: PrimitiveFormat = {
                module_file.module.name: [
                    (
                        direct_import.imported.name,
                        direct_import.line_number,
                        direct_import.line_contents,
                    )
                    for direct_import in imports_by_module[module_file.module]
                ]
                for module_file in found_package.module_files
            }
            primitives_map.update(primitives_map_for_found_package)

        serialized = json.dumps(primitives_map)
        data_cache_filename = self.file_system.join(
            self.cache_dir,
            self._namer.make_data_file_name(
                found_packages=self.found_packages,
                include_external_packages=self.include_external_packages,
                exclude_type_checking_imports=self.exclude_type_checking_imports,
            ),
        )
        self.file_system.write(data_cache_filename, serialized)
        logger.info(f"Wrote data cache file {data_cache_filename}.")

        # Write meta files.
        for found_package in self.found_packages:
            meta_filename = self.file_system.join(
                self.cache_dir, self._namer.make_meta_file_name(found_package)
            )
            mtime_map = {
                module_file.module.name: module_file.mtime
                for module_file in found_package.module_files
            }
            serialized_meta = json.dumps(mtime_map)
            self.file_system.write(meta_filename, serialized_meta)
            logger.info(f"Wrote meta cache file {meta_filename}.")

    def _build_mtime_map(self) -> None:
        self._mtime_map = self._read_mtime_map_files()

    def _read_mtime_map_files(self) -> Dict[str, float]:
        all_mtimes: Dict[str, float] = {}
        for found_package in self.found_packages:
            all_mtimes.update(self._read_mtime_map_file(found_package))
        return all_mtimes

    def _read_mtime_map_file(self, found_package: FoundPackage) -> Dict[str, float]:
        meta_cache_filename = self.file_system.join(
            self.cache_dir, self._namer.make_meta_file_name(found_package)
        )
        try:
            serialized = self.file_system.read(meta_cache_filename)
        except FileNotFoundError:
            logger.info(f"No cache file: {meta_cache_filename}.")
            return {}
        try:
            deserialized = json.loads(serialized)
            logger.info(f"Used cache meta file {meta_cache_filename}.")
            return deserialized
        except json.JSONDecodeError:
            logger.warning(f"Could not use corrupt cache file {meta_cache_filename}.")
            return {}

    def _build_data_map(self) -> None:
        self._data_map = self._read_data_map_file()

    def _read_data_map_file(self) -> Dict[Module, Set[DirectImport]]:
        data_cache_filename = self.file_system.join(
            self.cache_dir,
            self._namer.make_data_file_name(
                found_packages=self.found_packages,
                include_external_packages=self.include_external_packages,
                exclude_type_checking_imports=self.exclude_type_checking_imports,
            ),
        )
        try:
            serialized = self.file_system.read(data_cache_filename)
        except FileNotFoundError:
            logger.info(f"No cache file: {data_cache_filename}.")
            return {}

        # Deserialize to primitives.
        try:
            deserialized_json = json.loads(serialized)
            logger.info(f"Used cache data file {data_cache_filename}.")
        except json.JSONDecodeError:
            logger.warning(f"Could not use corrupt cache file {data_cache_filename}.")
            return {}

        primitives_map: PrimitiveFormat = self._to_primitives_data_map(deserialized_json)

        return {
            Module(name=name): {
                DirectImport(
                    importer=Module(name),
                    imported=Module(import_data[0]),
                    line_number=int(import_data[1]),  # type: ignore
                    line_contents=import_data[2],
                )
                for import_data in imports_data
            }
            for name, imports_data in primitives_map.items()
        }

    def _build_data_cache_filename(self, found_package: FoundPackage) -> str:
        return self.file_system.join(self.cache_dir, f"{found_package.name}.data.json")

    def _to_primitives_data_map(self, deserialized_json: object) -> PrimitiveFormat:
        """
        Convert the deserialized json from a data file to a narrower schema.

        Anything that doesn't fit the schema will be removed.
        """
        if not isinstance(deserialized_json, dict):
            return {}

        primitives_map: PrimitiveFormat = {}

        for key, value in deserialized_json.items():
            if not isinstance(key, str):
                continue
            if not isinstance(value, list):
                continue
            primitive_imports = []
            for deserialized_import in value:
                try:
                    [imported, line_number, line_contents] = deserialized_import
                except ValueError:
                    continue
                try:
                    primitive_imports.append(
                        (
                            str(imported),
                            int(line_number) if line_number else None,
                            str(line_contents),
                        )
                    )
                except TypeError:
                    continue

            primitives_map[key] = primitive_imports

        return primitives_map

    def _write_marker_files_if_not_already_there(self) -> None:
        marker_files_info = (
            (".gitignore", "# Automatically created by Grimp.\n*"),
            (
                "CACHEDIR.TAG",
                (
                    "Signature: 8a477f597d28d172789f06886806bc55\n"
                    "# This file is a cache directory tag automatically created by Grimp.\n"
                    "# For information about cache directory tags see https://bford.info/cachedir/"
                ),
            ),
        )

        for filename, contents in marker_files_info:
            full_filename = self.file_system.join(self.cache_dir, filename)
            if not self.file_system.exists(full_filename):
                self.file_system.write(full_filename, contents)
