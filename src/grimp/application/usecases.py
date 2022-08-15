"""
Use cases handle application logic.
"""
from typing import List, Set

from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.graph import AbstractImportGraph
from ..application.ports.importscanner import AbstractImportScanner
from ..application.ports.modulefinder import AbstractModuleFinder, FoundPackage
from ..application.ports.packagefinder import AbstractPackageFinder
from ..domain.valueobjects import Module
from .config import settings


def build_graph(
    package_name, *additional_package_names, include_external_packages: bool = False
) -> AbstractImportGraph:
    """
    Build and return an import graph for the supplied package name(s).

    Args:
        - package_name: the name of the top level package for which to build the graph.
        - additional_package_names: tuple of the
        - include_external_packages: whether to include any external packages in the graph.

    Examples:

        # Single package.
        graph = build_graph("mypackage")
        graph = build_graph("mypackage", include_external_packages=True)

        # Multiple packages.
        graph = build_graph("mypackage", "anotherpackage", "onemore")
        graph = build_graph(
            "mypackage", "anotherpackage", "onemore", include_external_packages=True,
        )
    """
    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    file_system: AbstractFileSystem = settings.FILE_SYSTEM
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    package_names = [package_name] + list(additional_package_names)
    modules: List[Module] = []
    found_packages: Set[FoundPackage] = set()
    _validate_package_names_are_strings(package_names)

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
        modules.extend(found_package.modules)

    import_scanner: AbstractImportScanner = settings.IMPORT_SCANNER_CLASS(
        file_system=file_system,
        found_packages=found_packages,
        include_external_packages=include_external_packages,
    )
    graph: AbstractImportGraph = settings.IMPORT_GRAPH_CLASS()

    # Scan each module for imports and add them to the graph.
    for module in modules:
        graph.add_module(module.name)
        for direct_import in import_scanner.scan_for_imports(module):
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


def _validate_package_names_are_strings(package_names: List[str]) -> None:
    for name in package_names:
        if not isinstance(name, str):
            raise TypeError(
                f"Package names must be strings, got {name.__class__.__name__}."
            )


def _is_external(module: Module, found_packages: Set[FoundPackage]) -> bool:
    package_modules = [Module(found_package.name) for found_package in found_packages]

    return not any(
        module.is_descendant_of(package_module) or module == package_module
        for package_module in package_modules
    )
