import os
import sys

import pytest

from grimp.application.config import settings
from grimp.adaptors.graph import NetworkXBackedImportGraph
from grimp.adaptors.modulefinder import ModuleFinder

from tests.adaptors.importscanner import BaseFakeImportScanner


@pytest.fixture(scope='module', autouse=True)
def configure_unit_tests():
    settings.configure(
        IMPORT_GRAPH_CLASS=NetworkXBackedImportGraph,
        MODULE_FINDER=ModuleFinder(),
        IMPORT_SCANNER_CLASS=BaseFakeImportScanner,
        FILE_SYSTEM=None,
    )

