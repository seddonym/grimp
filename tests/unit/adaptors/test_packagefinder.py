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
        ("testpackage", {str(assets / "testpackage")}),
        (
            "missingrootinitpackage",
            {
                str(assets / "missingrootinitpackage"),
            },
        ),
        (
            "missingrootinitpackage.one",
            {
                str(assets / "missingrootinitpackage" / "one"),
            },
        ),
        (
            "mynamespace.green",
            {str(assets / "namespacepackages" / "locationone" / "mynamespace" / "green")},
        ),
        (
            "mynamespace.blue",
            {str(assets / "namespacepackages" / "locationtwo" / "mynamespace" / "blue")},
        ),
        (
            "mynamespace",
            {
                str(assets / "namespacepackages" / "locationone" / "mynamespace"),
                str(assets / "namespacepackages" / "locationtwo" / "mynamespace"),
            },
        ),
    ),
)
def test_determine_package_directories(package, expected):
    assert (
        ImportLibPackageFinder().determine_package_directories(package, FileSystem()) == expected
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
def test_determine_package_directories_doesnt_support_non_top_level_modules(package):
    with pytest.raises(
        exceptions.NotATopLevelModule,
    ):
        ImportLibPackageFinder().determine_package_directories(package, FakeFileSystem())
