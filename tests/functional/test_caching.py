import shutil
import tempfile
from pathlib import Path

import pytest  # type: ignore

from grimp import build_graph

"""
For ease of reference, these are the imports of all the files:

cachingpackage: None
cachingpackage.one: None
cachingpackage.one.alpha: sys, pytest
cachingpackage.one.beta: cachingpackage.one.alpha
cachingpackage.one.gamma: cachingpackage.one.beta
cachingpackage.one.delta: None
cachingpackage.one.delta.blue: None
cachingpackage.two: None:
cachingpackage.two.alpha: cachingpackage.one.alpha
cachingpackage.two.beta: cachingpackage.one.alpha
cachingpackage.two.gamma: cachingpackage.two.beta, cachingpackage.utils
cachingpackage.utils: cachingpackage.one, cachingpackage.two.alpha

"""

CACHING_PATH = Path(__file__).parent.parent / "assets" / "caching"
PACKAGE_COPY_SOURCE = CACHING_PATH / "cachingpackage_to_copy"
PACKAGE_COPY_DESTINATION = CACHING_PATH / "cachingpackage"


@pytest.fixture
def copied_cachingpackage():
    """
    Makes a copy of caching package and then deletes it after the test.
    """
    if PACKAGE_COPY_DESTINATION.exists():
        shutil.rmtree(str(PACKAGE_COPY_DESTINATION))
    shutil.copytree(str(PACKAGE_COPY_SOURCE), str(PACKAGE_COPY_DESTINATION))
    yield "cachingpackage"
    shutil.rmtree(str(PACKAGE_COPY_DESTINATION))


def test_build_graph_uses_cache(copied_cachingpackage):
    with tempfile.TemporaryDirectory() as cache_dir:
        graph = build_graph("cachingpackage", cache_dir=cache_dir)

        real_import_details = [
            {
                "importer": "cachingpackage.two.alpha",
                "imported": "cachingpackage.one.alpha",
                "line_contents": "from ..one import alpha",
                "line_number": 1,
            },
        ]
        assert (
            graph.get_import_details(
                importer="cachingpackage.two.alpha",
                imported="cachingpackage.one.alpha",
            )
            == real_import_details
        )

        meta_file = Path(cache_dir) / "cachingpackage.meta.json"
        data_file = Path(cache_dir) / "Y2FjaGluZ3BhY2thZ2U-.data.json"

        assert meta_file.exists()
        assert data_file.exists()

        # Edit the contents of the cache.
        snippet = "from ..one import alpha"
        replacement = snippet + "  # Inserted by test"
        _manipulate_data_file(data_file, snippet, replacement)

        graph = build_graph("cachingpackage", cache_dir=cache_dir)

        # Reloading the graph should use the cache.
        manipulated_import_details = [
            {
                "importer": "cachingpackage.two.alpha",
                "imported": "cachingpackage.one.alpha",
                "line_contents": replacement,
                "line_number": 1,
            },
        ]
        assert (
            graph.get_import_details(
                importer="cachingpackage.two.alpha",
                imported="cachingpackage.one.alpha",
            )
            == manipulated_import_details
        )

        # Touch the file in question.
        (PACKAGE_COPY_DESTINATION / "two" / "alpha.py").touch()

        # Now shouldn't use the cache.
        graph = build_graph("cachingpackage", cache_dir=cache_dir)
        assert (
            graph.get_import_details(
                importer="cachingpackage.two.alpha",
                imported="cachingpackage.one.alpha",
            )
            == real_import_details
        )


def _manipulate_data_file(data_file: Path, snippet: str, replacement: str) -> None:
    with open(data_file, "r") as file:
        filedata = file.read()

    filedata = filedata.replace(snippet, replacement)

    with open(data_file, "w") as file:
        file.write(filedata)
