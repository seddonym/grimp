import pytest

from grimp import build_graph, exceptions

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
    graph = build_graph("testpackage")

    assert graph.modules == {
        "testpackage",
        "testpackage.one",
        "testpackage.one.alpha",
        "testpackage.one.beta",
        "testpackage.one.gamma",
        "testpackage.one.delta",
        "testpackage.one.delta.blue",
        "testpackage.two",
        "testpackage.two.alpha",
        "testpackage.two.beta",
        "testpackage.two.gamma",
        "testpackage.utils",
    }


def test_add_module():
    graph = build_graph("testpackage")
    number_of_modules = len(graph.modules)

    graph.add_module("foo")
    assert "foo" in graph.modules
    assert number_of_modules + 1 == len(graph.modules)


def test_remove_module():
    graph = build_graph("testpackage")
    number_of_modules = len(graph.modules)

    graph.remove_module("testpackage.two.alpha")
    assert "testpackage.two.alpha" not in graph.modules
    assert number_of_modules - 1 == len(graph.modules)


def test_add_and_remove_import():
    graph = build_graph("testpackage")
    a = "testpackage.one.delta.blue"
    b = "testpackage.two.alpha"

    assert not graph.direct_import_exists(importer=b, imported=a)

    graph.add_import(importer=b, imported=a)

    assert graph.direct_import_exists(importer=b, imported=a)

    graph.remove_import(importer=b, imported=a)

    assert not graph.direct_import_exists(importer=b, imported=a)


# Descendants
# -----------


def test_find_children():
    graph = build_graph("testpackage")

    assert graph.find_children("testpackage.one") == {
        "testpackage.one.alpha",
        "testpackage.one.beta",
        "testpackage.one.gamma",
        "testpackage.one.delta",
    }


def test_find_descendants():
    graph = build_graph("testpackage")

    assert graph.find_descendants("testpackage.one") == {
        "testpackage.one.alpha",
        "testpackage.one.beta",
        "testpackage.one.gamma",
        "testpackage.one.delta",
        "testpackage.one.delta.blue",
    }


# Direct imports
# --------------


def test_find_modules_directly_imported_by():
    graph = build_graph("testpackage")

    result = graph.find_modules_directly_imported_by("testpackage.utils")

    assert {"testpackage.one", "testpackage.two.alpha"} == result


def test_find_modules_that_directly_import():
    graph = build_graph("testpackage")

    result = graph.find_modules_that_directly_import("testpackage.one.alpha")

    assert {
        "testpackage.one.beta",
        "testpackage.two.alpha",
        "testpackage.two.beta",
    } == result


def test_direct_import_exists():
    graph = build_graph("testpackage")

    assert False is graph.direct_import_exists(
        importer="testpackage.one.alpha", imported="testpackage.two.alpha"
    )
    assert True is graph.direct_import_exists(
        importer="testpackage.two.alpha", imported="testpackage.one.alpha"
    )


def test_get_import_details():
    graph = build_graph("testpackage")

    assert [
        {
            "importer": "testpackage.utils",
            "imported": "testpackage.two.alpha",
            "line_number": 5,
            "line_contents": "from .two import alpha",
        }
    ] == graph.get_import_details(
        importer="testpackage.utils", imported="testpackage.two.alpha"
    )


# Indirect imports
# ----------------


class TestPathExists:
    def test_as_packages_false(self):
        graph = build_graph("testpackage")

        assert not graph.chain_exists(
            imported="testpackage.utils", importer="testpackage.one.alpha"
        )

        assert graph.chain_exists(
            imported="testpackage.one.alpha", importer="testpackage.utils"
        )

    def test_as_packages_true(self):
        graph = build_graph("testpackage")

        assert graph.chain_exists(
            imported="testpackage.one", importer="testpackage.utils", as_packages=True
        )

        assert not graph.chain_exists(
            imported="testpackage.utils", importer="testpackage.one", as_packages=True
        )


def test_find_shortest_chain():
    graph = build_graph("testpackage")

    assert (
        "testpackage.utils",
        "testpackage.two.alpha",
        "testpackage.one.alpha",
    ) == graph.find_shortest_chain(
        importer="testpackage.utils", imported="testpackage.one.alpha"
    )


def test_find_shortest_chains():
    graph = build_graph("testpackage")

    assert {
        ("testpackage.two.alpha", "testpackage.one.alpha"),
        ("testpackage.two.beta", "testpackage.one.alpha"),
        ("testpackage.two.gamma", "testpackage.utils", "testpackage.one"),
    } == graph.find_shortest_chains(
        importer="testpackage.two", imported="testpackage.one"
    )


class TestFindDownstreamModules:
    def test_as_package_false(self):
        graph = build_graph("testpackage")

        result = graph.find_downstream_modules("testpackage.one.alpha")

        assert {
            "testpackage.one.beta",
            "testpackage.one.gamma",
            "testpackage.two.alpha",
            "testpackage.two.beta",
            "testpackage.two.gamma",
            "testpackage.utils",
        } == result

    def test_as_package_true(self):
        graph = build_graph("testpackage")

        result = graph.find_downstream_modules("testpackage.one", as_package=True)

        assert {
            "testpackage.two.alpha",
            "testpackage.two.beta",
            "testpackage.two.gamma",
            "testpackage.utils",
        } == result


class TestFindUpstreamModules:
    def test_as_package_false(self):
        graph = build_graph("testpackage")

        assert graph.find_upstream_modules("testpackage.one.alpha") == set()

        assert graph.find_upstream_modules("testpackage.utils") == {
            "testpackage.one",
            "testpackage.two.alpha",
            "testpackage.one.alpha",
        }

    def test_as_package_true(self):
        graph = build_graph("testpackage")

        assert graph.find_upstream_modules("testpackage.two", as_package=True) == {
            "testpackage.one.alpha",
            "testpackage.utils",
            "testpackage.one",
        }


class TestSubpackageGraph:
    @pytest.mark.parametrize(
        "subpackage, expected_modules",
        (
            (
                "testpackage.one",
                {
                    "pytest",
                    "sys",
                    "testpackage.one",
                    "testpackage.one.alpha",
                    "testpackage.one.beta",
                    "testpackage.one.gamma",
                    "testpackage.one.delta",
                    "testpackage.one.delta.blue",
                },
            ),
            (
                "testpackage.two",
                {"testpackage.one.delta", "testpackage.one.delta.blue"},
            ),
            (
                "testpackage.one.alpha",
                exceptions.NotAPackage,
            ),
        ),
    )
    def test_modules(self, subpackage, expected_modules):
        graph = build_graph(subpackage)

        assert graph.modules == expected_modules

    @pytest.mark.parametrize(
        "subpackage, expected_modules",
        (
            (
                "testpackage.one",
                {
                    "testpackage.one",
                    "testpackage.one.alpha",
                    "testpackage.one.beta",
                    "testpackage.one.gamma",
                    "testpackage.one.delta",
                    "testpackage.one.delta.blue",
                },
                # TODO: how should we handle imports of other modules in the same root import package
                # e.g. an import of testpackage.two.alpha? As an external import, should that be included as
                # testpackage, testpackage.two, or testpackage.two.alpha? Or should it depend on whether it is
                # a namespace package or not?
                #
            ),
        ),
    )
    def test_modules_of_external_packages(self, subpackage, expected_modules):
        graph = build_graph(subpackage, include_external_packages=True)

        assert graph.modules == expected_modules

    @pytest.mark.parametrize(
        "module_to_add", ("testpackage", "testpackage.foo", "testpackage.one.foo")
    )
    def test_add_module(self, module_to_add):
        graph = build_graph("testpackage.one")

        graph.add_module(module_to_add)

        assert module_to_add in graph.module

    @pytest.mark.parametrize(
        "module_to_remove", ("testpackage.one", "testpackage.one.alpha")
    )
    def test_remove_module(self, module_to_remove):
        graph = build_graph("testpackage.one")

        graph.remove_module(module_to_remove)

        assert module_to_remove not in graph.modules

    def test_find_children(self):
        graph = build_graph("testpackage.one")

        assert graph.find_children("testpackage.one") == {
            "testpackage.one.alpha",
            "testpackage.one.beta",
            "testpackage.one.gamma",
            "testpackage.one.delta",
        }

    def test_find_descendants(self):
        graph = build_graph("testpackage.one")

        assert graph.find_descendants("testpackage.one") == {
            "testpackage.one.alpha",
            "testpackage.one.beta",
            "testpackage.one.gamma",
            "testpackage.one.delta",
            "testpackage.one.delta.blue",
        }

    def test_find_modules_directly_imported_by(self):
        graph = build_graph("testpackage.one")

        result = graph.find_modules_directly_imported_by("testpackage.one.beta")

        assert {"testpackage.one.alpha"} == result

    def test_find_modules_that_directly_import(self):
        graph = build_graph("testpackage.one")

        result = graph.find_modules_that_directly_import("testpackage.one.alpha")

        assert {"testpackage.one.beta"} == result
