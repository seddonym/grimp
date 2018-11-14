import pytest

from grimp import build_graph, Module, ImportPath, DirectImport

"""
For ease of reference, these are the imports of all the files:

testpackage: None
testpackage.one: None
testpackage.one.alpha: sys, pytest
testpackage.one.beta: testpackage.one.alpha
testpackage.one.gamma: testpackage.one.beta
testpackage.one.delta: None
testpackage.one.delta.blue: None
testpackage.two: None:
testpackage.two.alpha: testpackage.one.alpha
testpackage.two.beta: testpackage.one.alpha
testpackage.two.gamma: testpackage.two.beta, testpackage.utils
testpackage.utils: testpackage.one, testpackage.two.alpha

"""

def test_graph_modules():

    graph = build_graph('testpackage')

    assert graph.modules == {
        Module('testpackage'),
        Module('testpackage.one'),
        Module('testpackage.one.alpha'),
        Module('testpackage.one.beta'),
        Module('testpackage.one.gamma'),
        Module('testpackage.one.delta'),
        Module('testpackage.one.delta.blue'),
        Module('testpackage.two'),
        Module('testpackage.two.alpha'),
        Module('testpackage.two.beta'),
        Module('testpackage.two.gamma'),
        Module('testpackage.utils'),
    }


def test_find_children():
    graph = build_graph('testpackage')

    assert graph.find_children(Module('testpackage.one')) == {
        Module('testpackage.one.alpha'),
        Module('testpackage.one.beta'),
        Module('testpackage.one.gamma'),
        Module('testpackage.one.delta'),
    }


def test_find_descendants():
    graph = build_graph('testpackage')

    assert graph.find_descendants(Module('testpackage.one')) == {
        Module('testpackage.one.alpha'),
        Module('testpackage.one.beta'),
        Module('testpackage.one.gamma'),
        Module('testpackage.one.delta'),
        Module('testpackage.one.delta.blue'),
    }


def test_find_downstream_modules():
    graph = build_graph('testpackage')

    assert graph.find_downstream_modules(Module('testpackage.one.alpha')) == {
        Module('testpackage.one.beta'),
        Module('testpackage.one.gamma'),
        Module('testpackage.two.alpha'),
        Module('testpackage.two.beta'),
        Module('testpackage.two.gamma'),
        Module('testpackage.utils'),
    }


def test_find_upstream_modules():
    graph = build_graph('testpackage')

    assert graph.find_upstream_modules(Module('testpackage.one.alpha')) == set()

    assert graph.find_upstream_modules(Module('testpackage.utils')) == {
        Module('testpackage.one'),
        Module('testpackage.two.alpha'),
        Module('testpackage.one.alpha'),
    }


def test_find_shortest_path():
    graph = build_graph('testpackage')

    assert graph.find_shortest_path(
        upstream_module=Module('testpackage.utils'),
        downstream_module=Module('testpackage.one.alpha')
    ) == ImportPath(
        Module('testpackage.utils'),
        Module('testpackage.two.alpha'),
        Module('testpackage.one.alpha'),
    )


def test_find_modules_directly_imported_by():
    graph = build_graph('testpackage')

    assert graph.find_modules_directly_imported_by(Module('testpackage.utils')) == {
        Module('testpackage.one'), Module('testpackage.two.alpha'),
    }


def test_find_modules_that_directly_import():
    graph = build_graph('testpackage')

    assert graph.find_modules_that_directly_import(Module('testpackage.one.alpha')) == {
        Module('testpackage.one.beta'),
        Module('testpackage.two.alpha'),
        Module('testpackage.two.beta')
    }


def test_add_and_remove_import():
    graph = build_graph('testpackage')
    a = Module('testpackage.one.delta.blue')
    b = Module('testpackage.two.alpha')
    assert graph.find_downstream_modules(a) == set()

    graph.add_import(DirectImport(importer=b, imported=a))

    assert graph.find_downstream_modules(a) == {
        b, Module('testpackage.utils'), Module('testpackage.two.gamma')}

    graph.remove_import(DirectImport(importer=b, imported=a))

    assert graph.find_downstream_modules(a) == set()
