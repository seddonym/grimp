"""
Use cases handle application logic.
"""
from typing import Dict, Sequence, Set, Type, Union, cast

from ..application.ports import caching
from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.graph import ImportGraph
from ..application.ports.importscanner import AbstractImportScanner
from ..application.ports.modulefinder import AbstractModuleFinder, FoundPackage
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
    cache_dir: Union[str, Type[NotSupplied], None] = NotSupplied,
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
        file_system=file_system,
        include_external_packages=include_external_packages,
        exclude_type_checking_imports=exclude_type_checking_imports,
        cache_dir=cache_dir,
    )

    graph = _assemble_graph(found_packages, imports_by_module)

    return graph


def _find_packages(
    file_system: AbstractFileSystem, package_names: Sequence[object]
) -> Set[FoundPackage]:
    package_names = _validate_package_names_are_strings(package_names)

    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    found_packages: Set[FoundPackage] = set()

    for package_name in package_names:
        package_directory = package_finder.determine_package_directory(
            package_name=package_name, file_system=file_system
        )
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
    found_packages: Set[FoundPackage],
    file_system: AbstractFileSystem,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Union[str, Type[NotSupplied], None],
) -> Dict[Module, Set[DirectImport]]:
    imports_by_module: Dict[Module, Set[DirectImport]] = {}
    if cache_dir is not None:
        cache_dir_if_supplied = cache_dir if cache_dir != NotSupplied else None
        cache: caching.Cache = settings.CACHE_CLASS.setup(
            file_system=file_system,
            found_packages=found_packages,
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
            cache_dir=cache_dir_if_supplied,
        )
    import_scanner: AbstractImportScanner = settings.IMPORT_SCANNER_CLASS(
        file_system=file_system,
        found_packages=found_packages,
        include_external_packages=include_external_packages,
    )

    for found_package in found_packages:
        for module_file in found_package.module_files:
            module = module_file.module
            try:
                if cache_dir is None:
                    raise caching.CacheMiss
                direct_imports = cache.read_imports(module_file)
            except caching.CacheMiss:
                direct_imports = import_scanner.scan_for_imports(
                    module, exclude_type_checking_imports=exclude_type_checking_imports
                )
            imports_by_module[module] = direct_imports

    if cache_dir is not None:
        cache.write(imports_by_module)

    return imports_by_module


def _assemble_graph(
    found_packages: Set[FoundPackage],
    imports_by_module: Dict[Module, Set[DirectImport]],
) -> ImportGraph:
    graph: ImportGraph = settings.IMPORT_GRAPH_CLASS()
    for module, direct_imports in imports_by_module.items():
        graph.add_module(module.name)
        for direct_import in direct_imports:
            # Before we add the import, check to see if the imported module is in fact an
            # external module, and if so, tell the graph that it is a squashed module.
            graph.add_module(
                direct_import.imported.name,
                is_squashed=_is_external(direct_import.imported, found_packages),
            )

            graph.add_import(
                importer=direct_import.importer.name,
                imported=direct_import.imported.name,
                line_number=direct_import.line_number,
                line_contents=direct_import.line_contents,
            )
    return graph


def _is_external(module: Module, found_packages: Set[FoundPackage]) -> bool:
    package_modules = [Module(found_package.name) for found_package in found_packages]

    return not any(
        module.is_descendant_of(package_module) or module == package_module
        for package_module in package_modules
    )
