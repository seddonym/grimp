__all__ = ["build_graph"]

from .adaptors.caching import Cache
from .adaptors.filesystem import FileSystem
from .adaptors.graph import ImportGraph
from .adaptors.importscanner import ImportScanner
from .adaptors.modulefinder import ModuleFinder
from .adaptors.packagefinder import ImportLibPackageFinder
from .adaptors.timing import SystemClockTimer
from .application.config import settings
from .application.usecases import build_graph

settings.configure(
    MODULE_FINDER=ModuleFinder(),
    FILE_SYSTEM=FileSystem(),
    IMPORT_SCANNER_CLASS=ImportScanner,
    IMPORT_GRAPH_CLASS=ImportGraph,
    PACKAGE_FINDER=ImportLibPackageFinder(),
    CACHE_CLASS=Cache,
    TIMER=SystemClockTimer(),
)
