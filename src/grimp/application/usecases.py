"""
Use cases handle application logic.
"""
from ..application.ports.graph import AbstractImportGraph
from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.modulefinder import AbstractModuleFinder
from ..application.ports.importscanner import AbstractImportScanner
from ..application.ports.packagefinder import AbstractPackageFinder
from .config import settings


def build_graph(package_name) -> AbstractImportGraph:
    """
    Build and return an import graph for the supplied package name.
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
    )

    graph: AbstractImportGraph = settings.IMPORT_GRAPH_CLASS()

    # Scan each module for imports and add them to the graph.
    for module in modules:
        graph.add_module(module.name)
        for direct_import in import_scanner.scan_for_imports(module):
            graph.add_import(
                importer=direct_import.importer.name,
                imported=direct_import.imported.name,
                line_number=direct_import.line_number,
                line_contents=direct_import.line_contents,
            )

    return graph
