from grimp import build_graph

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

# Mechanics
# ---------


def test_modules():
    graph = build_graph('testpackage')

    assert graph.modules == {
        'testpackage',
        'testpackage.one',
        'testpackage.one.alpha',
        'testpackage.one.beta',
        'testpackage.one.gamma',
        'testpackage.one.delta',
        'testpackage.one.delta.blue',
        'testpackage.two',
        'testpackage.two.alpha',
        'testpackage.two.beta',
        'testpackage.two.gamma',
        'testpackage.utils',
    }


def test_add_module():
    graph = build_graph('testpackage')
    number_of_modules = len(graph.modules)

    graph.add_module('foo')
    assert 'foo' in graph.modules
    assert number_of_modules + 1 == len(graph.modules)


def test_remove_module():
    graph = build_graph('testpackage')
    number_of_modules = len(graph.modules)

    graph.remove_module('testpackage.two.alpha')
    assert 'testpackage.two.alpha' not in graph.modules
    assert number_of_modules - 1 == len(graph.modules)


def test_add_and_remove_import():
    graph = build_graph('testpackage')
    a = 'testpackage.one.delta.blue'
    b = 'testpackage.two.alpha'

    assert not graph.direct_import_exists(importer=b, imported=a)

    graph.add_import(importer=b, imported=a)

    assert graph.direct_import_exists(importer=b, imported=a)

    graph.remove_import(importer=b, imported=a)

    assert not graph.direct_import_exists(importer=b, imported=a)


# Descendants
# -----------

def test_find_children():
    graph = build_graph('testpackage')

    assert graph.find_children('testpackage.one') == {
        'testpackage.one.alpha',
        'testpackage.one.beta',
        'testpackage.one.gamma',
        'testpackage.one.delta',
    }


def test_find_descendants():
    graph = build_graph('testpackage')

    assert graph.find_descendants('testpackage.one') == {
        'testpackage.one.alpha',
        'testpackage.one.beta',
        'testpackage.one.gamma',
        'testpackage.one.delta',
        'testpackage.one.delta.blue',
    }


# Direct imports
# --------------

def test_find_modules_directly_imported_by():
    graph = build_graph('testpackage')

    result = graph.find_modules_directly_imported_by('testpackage.utils')

    assert {
        'testpackage.one', 'testpackage.two.alpha',
    } == result


def test_find_modules_that_directly_import():
    graph = build_graph('testpackage')

    result = graph.find_modules_that_directly_import(
        'testpackage.one.alpha')

    assert {
        'testpackage.one.beta',
        'testpackage.two.alpha',
        'testpackage.two.beta'
    } == result


def test_direct_import_exists():
    graph = build_graph('testpackage')

    assert False is graph.direct_import_exists(
        importer='testpackage.one.alpha',
        imported='testpackage.two.alpha',
    )
    assert True is graph.direct_import_exists(
        importer='testpackage.two.alpha',
        imported='testpackage.one.alpha',
    )


def test_get_import_details():
    graph = build_graph('testpackage')

    assert [
        {
            'importer': 'testpackage.utils',
            'imported': 'testpackage.two.alpha',
            'line_number': 5,
            'line_contents': 'from .two import alpha',
        }
    ] == graph.get_import_details(
        importer='testpackage.utils',
        imported='testpackage.two.alpha',
    )


# Indirect imports
# ----------------

class TestPathExists:
    def test_as_packages_false(self):
        graph = build_graph('testpackage')

        assert not graph.chain_exists(
            imported='testpackage.utils',
            importer='testpackage.one.alpha',
        )

        assert graph.chain_exists(
            imported='testpackage.one.alpha',
            importer='testpackage.utils',
        )

    def test_as_packages_true(self):
        graph = build_graph('testpackage')

        assert graph.chain_exists(
            imported='testpackage.one',
            importer='testpackage.utils',
            as_packages=True,
        )

        assert not graph.chain_exists(
            imported='testpackage.utils',
            importer='testpackage.one',
            as_packages=True,
        )


def test_find_shortest_chain():
    graph = build_graph('testpackage')

    assert (
        'testpackage.utils',
        'testpackage.two.alpha',
        'testpackage.one.alpha',
    ) == graph.find_shortest_chain(
        importer='testpackage.utils',
        imported='testpackage.one.alpha',
    )


class TestFindDownstreamModules:
    def test_as_package_false(self):
        graph = build_graph('testpackage')

        result = graph.find_downstream_modules('testpackage.one.alpha')

        assert {
            'testpackage.one.beta',
            'testpackage.one.gamma',
            'testpackage.two.alpha',
            'testpackage.two.beta',
            'testpackage.two.gamma',
            'testpackage.utils',
        } == result

    def test_as_package_true(self):
        graph = build_graph('testpackage')

        result = graph.find_downstream_modules('testpackage.one', as_package=True)

        assert {
            'testpackage.two.alpha',
            'testpackage.two.beta',
            'testpackage.two.gamma',
            'testpackage.utils',
        } == result


class TestFindUpstreamModules:
    def test_as_package_false(self):
        graph = build_graph('testpackage')

        assert graph.find_upstream_modules('testpackage.one.alpha') == set()

        assert graph.find_upstream_modules('testpackage.utils') == {
            'testpackage.one',
            'testpackage.two.alpha',
            'testpackage.one.alpha',
        }

    def test_as_package_true(self):
        graph = build_graph('testpackage')

        assert graph.find_upstream_modules('testpackage.two', as_package=True) == {
            'testpackage.one.alpha',
            'testpackage.utils',
            'testpackage.one',
        }
