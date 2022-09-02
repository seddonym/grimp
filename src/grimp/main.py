__all__ = ["build_graph"]

from .application.usecases import build_graph
from .application.config import settings
from .adaptors.importscanner import ImportScanner
from .adaptors.modulefinder import ModuleFinder
from .adaptors.filesystem import FileSystem
from .adaptors.graph import ImportGraph
from .adaptors.rustgraph import ImportGraph as RustImportGraph

from .adaptors.packagefinder import ImportLibPackageFinder


settings.configure(
    MODULE_FINDER=ModuleFinder(),
    FILE_SYSTEM=FileSystem(),
    IMPORT_SCANNER_CLASS=ImportScanner,
    IMPORT_GRAPH_CLASS=RustImportGraph,
    PACKAGE_FINDER=ImportLibPackageFinder(),
)
