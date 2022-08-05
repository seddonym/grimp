import pytest

from grimp import build_graph, exceptions

"""
For ease of reference, these are the imports of all the files:

mynamespace.yellow: mynamespace.green.alpha
mynamespace.green: mynamespace.yellow
mynamespace.blue.alpha: mynamespace.blue.beta, mynamespace.green.alpha
"""

# Mechanics
# ---------


def test_build_graph_for_namespace():
    with pytest.raises(exceptions.NamespacePackageEncountered):
        build_graph("mynamespace")


GREEN_MODULES = {"mynamespace.green", "mynamespace.green.alpha"}
BLUE_MODULES = {"mynamespace.blue", "mynamespace.blue.alpha", "mynamespace.blue.beta"}


@pytest.mark.parametrize(
    "package, expected_modules",
    (
        (
            "mynamespace.green",
            GREEN_MODULES,
        ),
        (
            "mynamespace.blue",
            BLUE_MODULES,
        ),
    ),
)
def test_modules_for_namespace_child(package, expected_modules):
    graph = build_graph(package)

    assert graph.modules == expected_modules


def test_modules_for_multiple_namespace_children():
    graph = build_graph("mynamespace.green", "mynamespace.blue")

    assert graph.modules == GREEN_MODULES | BLUE_MODULES


def test_import_within_namespace_child():
    graph = build_graph("mynamespace.blue")

    assert graph.direct_import_exists(
        importer="mynamespace.blue.alpha", imported="mynamespace.blue.beta"
    )


def test_import_between_namespace_children():
    graph = build_graph("mynamespace.blue", "mynamespace.green")

    assert graph.direct_import_exists(
        importer="mynamespace.blue.alpha", imported="mynamespace.green.alpha"
    )
