import pytest  # type: ignore

from grimp.application.config import settings
from grimp.application.graph import ImportGraph
from grimp.adaptors.modulefinder import ModuleFinder


@pytest.fixture(scope="module", autouse=True)
def configure_unit_tests():
    settings.configure(
        IMPORT_GRAPH_CLASS=ImportGraph,
        MODULE_FINDER=ModuleFinder(),
        FILE_SYSTEM=None,
    )
