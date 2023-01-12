import re
from pathlib import Path

import pytest  # type: ignore

from grimp import exceptions
from grimp.adaptors.packagefinder import ImportLibPackageFinder
from grimp.adaptors.filesystem import FileSystem
from tests.adaptors.filesystem import FakeFileSystem

assets = (Path(__file__).parent.parent.parent / "assets").resolve()


@pytest.mark.parametrize(
    "package, expected",
    (
        ("testpackage", assets / "testpackage"),
        (
            "mynamespace.green",
            assets / "namespacepackages" / "locationone" / "mynamespace" / "green",
        ),
        (
            "mynamespace.blue",
            assets / "namespacepackages" / "locationtwo" / "mynamespace" / "blue",
        ),
    ),
)
def test_determine_package_directory(package, expected):
    assert ImportLibPackageFinder().determine_package_directory(
        package, FileSystem()
    ) == str(expected)


def test_determine_package_directory_doesnt_support_namespace_packages():
    with pytest.raises(
        exceptions.NamespacePackageEncountered,
        match=re.escape(
            "Package 'mynamespace' is a namespace package (see PEP 420). Try specifying the"
            " portion name instead. If you are not intentionally using "
            "namespace packages, adding an __init__.py file should fix the problem."
        ),
    ):
        ImportLibPackageFinder().determine_package_directory(
            "mynamespace", FakeFileSystem()
        )


@pytest.mark.parametrize(
    "package",
    (
        "testpackage.one",
        "testpackage.one.alpha",
        "testpackage.one.delta",
        "mynamespace.green.alpha",
        "mynamespace.yellow",
    ),
)
def test_determine_package_directory_doesnt_support_non_top_level_modules(package):
    with pytest.raises(
        exceptions.NotATopLevelModule,
    ):
        ImportLibPackageFinder().determine_package_directory(package, FakeFileSystem())
