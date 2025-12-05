"""
Use cases handle application logic.
"""

from typing import cast
import itertools
from collections.abc import Sequence, Iterable

from .scanning import scan_imports
from ..application.ports import caching
from ..application.ports.filesystem import AbstractFileSystem, BasicFileSystem
from ..application.graph import ImportGraph
from ..application.ports.modulefinder import AbstractModuleFinder, FoundPackage, ModuleFile
from ..application.ports.packagefinder import AbstractPackageFinder
from ..domain.valueobjects import DirectImport, Module
from .config import settings


class NotSupplied:
    pass


def build_graph(
    package_name,
    *additional_package_names,
    include_external_packages: bool = False,
    exclude_type_checking_imports: bool = False,
    cache_dir: str | type[NotSupplied] | None = NotSupplied,
) -> ImportGraph:
    """
    Build and return an import graph for the supplied package name(s).

    Args:
        - package_name: the name of the top level package for which to build the graph.
        - additional_package_names: tuple of the
        - include_external_packages: whether to include any external packages in the graph.
        - exclude_type_checking_imports: whether to exclude imports made in type checking guards.
        - cache_dir: The directory to use for caching the graph.
    Examples:

        # Single package.
        graph = build_graph("mypackage")
        graph = build_graph("mypackage", include_external_packages=True)
        graph = build_graph("mypackage", exclude_type_checking_imports=True)

        # Multiple packages.
        graph = build_graph("mypackage", "anotherpackage", "onemore")
        graph = build_graph(
            "mypackage", "anotherpackage", "onemore", include_external_packages=True,
        )
    """
    file_system: AbstractFileSystem = settings.FILE_SYSTEM

    found_packages = _find_packages(
        file_system=file_system,
        package_names=[package_name] + list(additional_package_names),
    )

    imports_by_module = _scan_packages(
        found_packages=found_packages,
        file_system=file_system.convert_to_basic(),
        include_external_packages=include_external_packages,
        exclude_type_checking_imports=exclude_type_checking_imports,
        cache_dir=cache_dir,
    )

    graph = _assemble_graph(found_packages, imports_by_module)

    return graph


def _find_packages(
    file_system: AbstractFileSystem, package_names: Sequence[object]
) -> set[FoundPackage]:
    package_names = _validate_package_names_are_strings(package_names)

    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    found_packages: set[FoundPackage] = set()

    for package_name in package_names:
        package_directories = package_finder.determine_package_directories(
            package_name=package_name, file_system=file_system
        )
        for package_directory in package_directories:
            found_package = module_finder.find_package(
                package_name=package_name,
                package_directory=package_directory,
                file_system=file_system,
            )
            found_packages.add(found_package)

    return found_packages


def _validate_package_names_are_strings(
    package_names: Sequence[object],
) -> Sequence[str]:
    for name in package_names:
        if not isinstance(name, str):
            raise TypeError(f"Package names must be strings, got {name.__class__.__name__}.")
    return cast(Sequence[str], package_names)


def _scan_packages(
    found_packages: set[FoundPackage],
    file_system: BasicFileSystem,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: str | type[NotSupplied] | None,
) -> dict[Module, set[DirectImport]]:
    if cache_dir is not None:
        cache_dir_if_supplied = cache_dir if cache_dir != NotSupplied else None
        cache: caching.Cache = settings.CACHE_CLASS.setup(
            file_system=file_system,
            found_packages=found_packages,
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
            cache_dir=cache_dir_if_supplied,
        )

    module_files_to_scan = {
        module_file
        for found_package in found_packages
        for module_file in found_package.module_files
    }

    imports_by_module_file: dict[ModuleFile, set[DirectImport]] = {}

    if cache_dir is not None:
        imports_by_module_file.update(_read_imports_from_cache(module_files_to_scan, cache=cache))

    remaining_module_files_to_scan = module_files_to_scan.difference(imports_by_module_file)
    if remaining_module_files_to_scan:
        imports_by_module_file.update(
            scan_imports(
                remaining_module_files_to_scan,
                found_packages=found_packages,
                include_external_packages=include_external_packages,
                exclude_type_checking_imports=exclude_type_checking_imports,
            )
        )

    imports_by_module: dict[Module, set[DirectImport]] = {
        k.module: v for k, v in imports_by_module_file.items()
    }

    if cache_dir is not None:
        cache.write(imports_by_module)

    return imports_by_module


def _assemble_graph(
    found_packages: set[FoundPackage],
    imports_by_module: dict[Module, set[DirectImport]],
) -> ImportGraph:
    graph: ImportGraph = settings.IMPORT_GRAPH_CLASS()

    for namespace_package in itertools.chain.from_iterable(
        found_package.namespace_packages for found_package in found_packages
    ):
        graph.add_module(namespace_package)

    package_modules = {Module(found_package.name) for found_package in found_packages}

    for module, direct_imports in imports_by_module.items():
        graph.add_module(module.name)
        for direct_import in direct_imports:
            # Before we add the import, check to see if the imported module is in fact an
            # external module, and if so, tell the graph that it is a squashed module.
            graph.add_module(
                direct_import.imported.name,
                is_squashed=_is_external(direct_import.imported, package_modules),
            )

            graph.add_import(
                importer=direct_import.importer.name,
                imported=direct_import.imported.name,
                line_number=direct_import.line_number,
                line_contents=direct_import.line_contents,
            )
    return graph


def _is_external(module: Module, package_modules: set[Module]) -> bool:
    return not any(
        module.is_descendant_of(package_module) or module == package_module
        for package_module in package_modules
    )


def _read_imports_from_cache(
    module_files: Iterable[ModuleFile], *, cache: caching.Cache
) -> dict[ModuleFile, set[DirectImport]]:
    imports_by_module_file: dict[ModuleFile, set[DirectImport]] = {}
    for module_file in module_files:
        try:
            direct_imports = cache.read_imports(module_file)
        except caching.CacheMiss:
            continue
        else:
            imports_by_module_file[module_file] = direct_imports
    return imports_by_module_file
