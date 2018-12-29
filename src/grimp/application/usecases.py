"""
Use cases handle application logic.
"""
from ..application.ports.graph import AbstractImportGraph
from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.modulefinder import AbstractModuleFinder
from ..application.ports.importscanner import AbstractImportScanner
from ..application.ports.packagefinder import AbstractPackageFinder
from .config import settings


def build_graph(package_name, include_external_packages: bool = False) -> AbstractImportGraph:
    """
    Build and return an import graph for the supplied package name.

    Args:
        - package_name: the name of the top level package for which to build the graph.
        - include_external_packages: whether to include any external packages in the graph.
    """
    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    file_system: AbstractFileSystem = settings.FILE_SYSTEM
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    package_directory = package_finder.determine_package_directory(
        package_name=package_name,
        file_system=file_system,
    )

    # Build a list of all the Python modules in the package.
    modules = module_finder.find_modules(
        package_name=package_name,
        package_directory=package_directory,
        file_system=file_system,
    )

    import_scanner: AbstractImportScanner = settings.IMPORT_SCANNER_CLASS(
        modules=modules,
        package_directory=package_directory,
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    graph: AbstractImportGraph = settings.IMPORT_GRAPH_CLASS()

    # Scan each module for imports and add them to the graph.
    for module in modules:
        graph.add_module(module.name)
        for direct_import in import_scanner.scan_for_imports(module):
            # Before we add the import, check to see if the imported module is in fact an
            # external module, and if so, tell the graph that it is a squashed module.
            is_external = (direct_import.imported.package_name != package_name)
            graph.add_module(direct_import.imported.name, is_squashed=is_external)

            graph.add_import(
                importer=direct_import.importer.name,
                imported=direct_import.imported.name,
                line_number=direct_import.line_number,
                line_contents=direct_import.line_contents,
            )

    return graph
