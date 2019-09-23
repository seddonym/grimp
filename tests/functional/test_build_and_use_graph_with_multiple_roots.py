import pytest
from grimp import build_graph

"""
For ease of reference, these are the imports of all the files:

rootpackageblue: None
rootpackageblue.one: 
rootpackageblue.one.alpha: sys, pytest
rootpackageblue.two: from .one.alpha import BAR
rootpackageblue.three: import rootpackageblue.two

rootpackagegreen: None
rootpackagegreen.one: None
rootpackagegreen.two: from rootpackageblue.one import alpha, from . import one
"""

# The order of the root packages supplied shouldn't affect the graph.
PACKAGES_IN_DIFFERENT_ORDERS = (
    ("rootpackageblue", "rootpackagegreen",),
    ("rootpackagegreen", "rootpackageblue"),
)


@pytest.mark.parametrize("root_packages", PACKAGES_IN_DIFFERENT_ORDERS)
class TestBuildGraph:
    def test_graph_has_correct_modules_regardless_of_package_order(self, root_packages):
        graph = build_graph(*root_packages)

        assert graph.modules == {
            "rootpackageblue",
            "rootpackageblue.one",
            "rootpackageblue.one.alpha",
            "rootpackageblue.two",
            "rootpackageblue.three",
            "rootpackagegreen",
            "rootpackagegreen.one",
            "rootpackagegreen.two",
        }

    def test_stores_import_within_package(self, root_packages):
        graph = build_graph(*root_packages)

        assert [
            {
                "importer": "rootpackageblue.two",
                "imported": "rootpackageblue.one.alpha",
                "line_number": 1,
                "line_contents": "from .one.alpha import BAR",
            }
        ] == graph.get_import_details(
            importer="rootpackageblue.two", imported="rootpackageblue.one.alpha"
        )

    def test_stores_import_between_root_packages(self, root_packages):
        graph = build_graph(*root_packages)

        assert [
            {
                "importer": "rootpackagegreen.two",
                "imported": "rootpackageblue.one.alpha",
                "line_number": 1,
                "line_contents": "from rootpackageblue.one import alpha",
            }
        ] == graph.get_import_details(
            importer="rootpackagegreen.two", imported="rootpackageblue.one.alpha"
        )

