from pathlib import Path

import pytest

from grimp.adaptors.packagefinder import ImportLibPackageFinder
from tests.adaptors.filesystem import FakeFileSystem

assets = (Path(__file__).parent.parent.parent / "assets").resolve()


@pytest.mark.parametrize(
    "package, expected", (("testpackage", assets / "testpackage"),)
)
def test_determine_package_directory(package, expected):
    assert (
        ImportLibPackageFinder().determine_package_directory(package, FakeFileSystem())
        == str(expected)
    )
