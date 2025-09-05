from typing import Collection, Set, Dict

from grimp import _rustgrimp as rust  # type: ignore[attr-defined]
from grimp.domain.valueobjects import DirectImport, Module
from grimp.application.config import settings
from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.application.ports.modulefinder import ModuleFile, FoundPackage


def scan_imports(
    module_files: Collection[ModuleFile],
    *,
    found_packages: Set[FoundPackage],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> Dict[ModuleFile, Set[DirectImport]]:
    file_system: AbstractFileSystem = settings.FILE_SYSTEM
    basic_file_system = file_system.convert_to_basic()
    imports_by_module: dict[Module, set[DirectImport]] = rust.scan_for_imports(
        module_files=tuple(module_files),
        found_packages=found_packages,
        # Ensure that the passed exclude_type_checking_imports is definitely a boolean,
        # otherwise the Rust class will error.
        include_external_packages=bool(include_external_packages),
        exclude_type_checking_imports=exclude_type_checking_imports,
        file_system=basic_file_system,
    )
    return {module_file: imports_by_module[module_file.module] for module_file in module_files}
