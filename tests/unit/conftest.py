import pytest  # type: ignore

from grimp.application.config import settings
from grimp.adaptors.graph import ImportGraph
from grimp.adaptors.modulefinder import ModuleFinder
from grimp.adaptors.importscanner import ImportScanner


@pytest.fixture(scope='module', autouse=True)
def configure_unit_tests():
    settings.configure(
        IMPORT_GRAPH_CLASS=ImportGraph,
        MODULE_FINDER=ModuleFinder(),
        IMPORT_SCANNER_CLASS=ImportScanner,
        FILE_SYSTEM=None,
    )
