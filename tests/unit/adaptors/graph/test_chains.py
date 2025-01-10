from typing import Set, Tuple

import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph


@pytest.mark.parametrize(
    "module, as_package, expected_result",
    (
        ("foo.a", False, {"foo.b", "foo.c", "foo.a.d", "foo.b.e"}),
        ("foo.b.e", False, set()),
        ("foo.a", True, {"foo.b", "foo.c", "foo.b.e", "foo.b.g"}),
        ("foo.b.e", True, set()),
        ("bar", True, {"foo.a.d", "foo.b.e"}),
        ("bar", False, {"foo.a.d", "foo.b.e"}),
    ),
)
def test_find_downstream_modules(module, as_package, expected_result):
    graph = ImportGraph()
    a, b, c = "foo.a", "foo.b", "foo.c"
    d, e, f = "foo.a.d", "foo.b.e", "foo.a.f"
    g = "foo.b.g"
    external = "bar"

    graph.add_module(external, is_squashed=True)

    graph.add_import(importer=b, imported=a)
    graph.add_import(importer=c, imported=a)
    graph.add_import(importer=d, imported=c)
    graph.add_import(importer=e, imported=d)
    graph.add_import(importer=b, imported=f)
    graph.add_import(importer=g, imported=f)
    graph.add_import(importer=d, imported=external)

    assert expected_result == graph.find_downstream_modules(module, as_package=as_package)


@pytest.mark.parametrize(
    "module, as_package, expected_result",
    (
        ("foo.d", False, {"foo.d.c", "foo.a"}),
        ("foo.b.g", False, set()),
        # Note: foo.d.c is not included in the upstreams because that's internal to the package.
        ("foo.d", True, {"foo.a", "foo.a.f", "foo.b.g"}),
        ("foo.b.g", True, set()),
        ("bar", True, {"foo.a.f", "foo.b.g"}),
        ("bar", False, {"foo.a.f", "foo.b.g"}),
    ),
)
def test_find_upstream_modules(module, as_package, expected_result):
    graph = ImportGraph()
    a, b, c = "foo.a", "foo.d.b", "foo.d.c"
    d, e, f = "foo.d", "foo.c.e", "foo.a.f"
    g = "foo.b.g"
    external = "bar"

    graph.add_module(external, is_squashed=True)

    graph.add_import(importer=b, imported=a)
    graph.add_import(importer=c, imported=a)
    graph.add_import(importer=d, imported=c)
    graph.add_import(importer=e, imported=d)
    graph.add_import(importer=b, imported=f)
    graph.add_import(importer=f, imported=g)
    graph.add_import(importer=external, imported=f)

    assert expected_result == graph.find_upstream_modules(module, as_package=as_package)


class TestFindShortestChain:
    def test_find_shortest_chain_when_exists(self):
        graph = ImportGraph()
        a, b, c = "foo", "bar", "baz"
        d, e, f = "long", "way", "around"

        # Add short path.
        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=b, imported=c)

        # Add longer path.
        graph.add_import(importer=a, imported=d)
        graph.add_import(importer=d, imported=e)
        graph.add_import(importer=e, imported=f)
        graph.add_import(importer=f, imported=c)

        assert (a, b, c) == graph.find_shortest_chain(importer=a, imported=c)

    def test_find_shortest_chain_returns_direct_import_when_exists(self):
        graph = ImportGraph()
        a, b = "foo", "bar"
        d, e, f = "long", "way", "around"

        # Add short path.
        graph.add_import(importer=a, imported=b)

        # Add longer path.
        graph.add_import(importer=a, imported=d)
        graph.add_import(importer=d, imported=e)
        graph.add_import(importer=e, imported=f)
        graph.add_import(importer=f, imported=b)

        assert (a, b) == graph.find_shortest_chain(importer=a, imported=b)

    def test_find_shortest_chain_returns_none_if_not_exists(self):
        graph = ImportGraph()
        a, b, c = "foo", "bar", "baz"

        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=b, imported=c)

        assert None is graph.find_shortest_chain(importer=c, imported=a)

    def test_raises_value_error_if_importer_not_present(self):
        graph = ImportGraph()

        with pytest.raises(ValueError, match="Module foo is not present in the graph."):
            graph.find_shortest_chain(importer="foo", imported="bar")

    def test_raises_value_error_if_imported_not_present(self):
        graph = ImportGraph()
        graph.add_module("foo")

        with pytest.raises(ValueError, match="Module bar is not present in the graph."):
            graph.find_shortest_chain(importer="foo", imported="bar")

    def test_find_shortest_chain_copes_with_cycle(self):
        graph = ImportGraph()
        a, b, c, d, e = "blue", "green", "orange", "yellow", "purple"

        # Add path with some cycles.
        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=b, imported=a)
        graph.add_import(importer=b, imported=c)
        graph.add_import(importer=c, imported=d)
        graph.add_import(importer=d, imported=b)
        graph.add_import(importer=d, imported=e)
        graph.add_import(importer=e, imported=d)

        assert (a, b, c, d, e) == graph.find_shortest_chain(importer=a, imported=e)

    def test_demonstrate_nondeterminism_of_equal_chains(self):
        """
        This test demonstrates that, unfortunately, find_shortest_chain
        does not have deterministic behaviour when two chains of equal
        length exist in the graph.
        """
        graph = ImportGraph()
        source, destination = "source", "destination"
        a, b, c = "a", "b", "c"
        d, e, f = "d", "e", "f"

        # Add first chain.
        graph.add_import(importer=source, imported=a)
        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=b, imported=c)
        graph.add_import(importer=c, imported=destination)

        # Add a second chain of equal length.
        graph.add_import(importer=source, imported=d)
        graph.add_import(importer=d, imported=e)
        graph.add_import(importer=e, imported=f)
        graph.add_import(importer=f, imported=destination)

        result = graph.find_shortest_chain(importer=source, imported=destination)

        one_chain = (source, a, b, c, destination)
        other_chain = (source, d, e, f, destination)
        assert (result == one_chain) or (result == other_chain)


class TestFindShortestChains:
    @pytest.mark.parametrize(
        "importer, imported",
        (
            ("green", "green.one"),
            ("green.one", "green"),
        ),
    )
    def test_modules_with_shared_descendants_raises_value_error_when_as_packages_true(
        self, importer: str, imported: str
    ):
        graph = ImportGraph()
        graph.add_module(importer)
        graph.add_module(imported)

        with pytest.raises(ValueError, match="Modules have shared descendants."):
            graph.find_shortest_chains(importer=importer, imported=imported, as_packages=True)

    @pytest.mark.parametrize(
        "importer, imported",
        (
            ("green", "green.one"),
            ("green.one", "green"),
        ),
    )
    def test_modules_with_shared_descendants_allowed_when_as_packages_false(
        self, importer: str, imported: str
    ):
        graph = ImportGraph()
        middle = "middle"
        graph.add_import(importer=importer, imported=middle)
        graph.add_import(importer=middle, imported=imported)

        result = graph.find_shortest_chains(
            importer=importer, imported=imported, as_packages=False
        )

        assert result == {(importer, middle, imported)}

    @pytest.mark.parametrize("as_packages", (False, True))
    def test_top_level_import(self, as_packages: bool):
        graph = ImportGraph()
        graph.add_import(importer="green", imported="blue")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == {("green", "blue")}

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, set()),
            (True, {("green.foo", "blue.bar")}),
        ),
    )
    def test_first_level_child_import(self, as_packages: bool, expected_result: Set[Tuple]):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")
        graph.add_import(importer="green.foo", imported="blue.bar")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    @pytest.mark.parametrize("as_packages", (False, True))
    def test_no_results_in_reverse_direction(self, as_packages: bool):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")
        graph.add_import(importer="green.foo", imported="blue.bar")

        result = graph.find_shortest_chains(
            importer="blue", imported="green", as_packages=as_packages
        )

        assert result == set()

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, set()),
            (True, {("green.foo.one", "blue.bar.two")}),
        ),
    )
    def test_grandchildren_import(self, as_packages: bool, expected_result: Set[Tuple]):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")
        graph.add_import(importer="green.foo.one", imported="blue.bar.two")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, set()),
            (True, {("green.foo", "blue")}),
        ),
    )
    def test_import_between_child_and_top_level(
        self, as_packages: bool, expected_result: Set[Tuple]
    ):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_import(importer="green.foo", imported="blue")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, set()),
            (True, {("green", "blue.foo")}),
        ),
    )
    def test_import_between_top_level_and_child(
        self, as_packages: bool, expected_result: Set[Tuple]
    ):
        graph = ImportGraph()
        graph.add_module("blue")
        graph.add_import(importer="green", imported="blue.foo")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, {("green", "yellow", "red", "blue")}),
            (
                True,
                {
                    ("green.indirect", "purple", "blue.foo"),
                    ("green", "yellow", "red", "blue"),
                },
            ),
        ),
    )
    def test_short_indirect_import(self, as_packages: bool, expected_result: Set[Tuple]):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")

        # Test cases for indirect import as package
        graph.add_import(importer="green.indirect", imported="purple")
        graph.add_import(importer="purple", imported="blue.foo")

        # Test cases for indirect import as module
        graph.add_import(importer="green", imported="yellow")
        graph.add_import(importer="yellow", imported="red")
        graph.add_import(importer="red", imported="blue")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    @pytest.mark.parametrize(
        "as_packages, expected_result",
        (
            (False, {("green", "red.three", "red.two", "red.one", "blue")}),
            (
                True,
                {
                    ("green.baz", "yellow.three", "yellow.two", "yellow.one", "blue.foo"),
                    ("green", "red.three", "red.two", "red.one", "blue"),
                },
            ),
        ),
    )
    def test_long_indirect_import(self, as_packages: bool, expected_result: Set[Tuple]):
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")

        # Test cases for indirect import as package
        graph.add_import(importer="green.baz", imported="yellow.three")
        graph.add_import(importer="yellow.three", imported="yellow.two")
        graph.add_import(importer="yellow.two", imported="yellow.one")
        graph.add_import(importer="yellow.one", imported="blue.foo")

        # Test cases for indirect import as module
        graph.add_import(importer="green", imported="red.three")
        graph.add_import(importer="red.three", imported="red.two")
        graph.add_import(importer="red.two", imported="red.one")
        graph.add_import(importer="red.one", imported="blue")

        result = graph.find_shortest_chains(
            importer="green", imported="blue", as_packages=as_packages
        )

        assert result == expected_result

    def test_chains_within_packages_are_not_included(self):
        graph = ImportGraph()

        graph.add_module("importer_package")
        graph.add_module("imported_package")

        # Chain via importer package.
        graph.add_import(importer="importer_package.one", imported="importer_package.two")
        graph.add_import(importer="importer_package.two", imported="importer_package.three")
        graph.add_import(importer="importer_package.three", imported="imported_package.four")
        graph.add_import(importer="imported_package.four", imported="imported_package.five")
        graph.add_import(importer="imported_package.five", imported="imported_package.six")

        result = graph.find_shortest_chains(
            importer="importer_package", imported="imported_package"
        )

        assert result == {("importer_package.three", "imported_package.four")}

    def test_chains_via_importer_package_dont_stop_longer_chains_being_included(self):
        graph = ImportGraph()

        graph.add_module("green")
        graph.add_module("blue")

        # Chain via importer package.
        graph.add_import(importer="green.foo", imported="blue.foo")
        graph.add_import(importer="green.baz", imported="green.foo")

        # Long indirect import.
        graph.add_import(importer="green.baz", imported="yellow.three")
        graph.add_import(importer="yellow.three", imported="yellow.two")
        graph.add_import(importer="yellow.two", imported="yellow.one")
        graph.add_import(importer="yellow.one", imported="blue.bar")

        result = graph.find_shortest_chains(importer="green", imported="blue")
        assert result == {
            ("green.foo", "blue.foo"),
            ("green.baz", "yellow.three", "yellow.two", "yellow.one", "blue.bar"),
        }

    def test_chains_that_reenter_importer_package_dont_stop_longer_chains_being_included(
        self,
    ):
        graph = ImportGraph()

        graph.add_module("green")
        graph.add_module("blue")

        # Chain that reenters importer package.
        graph.add_import(importer="green.baz", imported="brown")
        graph.add_import(importer="brown", imported="green.foo")
        graph.add_import(importer="green.foo", imported="blue.foo")

        # Long indirect import.
        graph.add_import(importer="green.baz", imported="yellow.three")
        graph.add_import(importer="yellow.three", imported="yellow.two")
        graph.add_import(importer="yellow.two", imported="yellow.one")
        graph.add_import(importer="yellow.one", imported="blue.foo")

        result = graph.find_shortest_chains(importer="green", imported="blue")
        assert result == {
            ("green.foo", "blue.foo"),
            ("green.baz", "yellow.three", "yellow.two", "yellow.one", "blue.foo"),
        }

    def test_chains_that_reenter_imported_package_dont_stop_longer_chains_being_included(
        self,
    ):
        graph = ImportGraph()

        graph.add_module("green")
        graph.add_module("blue")

        # Chain that reenters imported package.
        graph.add_import(importer="green.foo", imported="blue.foo")
        graph.add_import(importer="blue.foo", imported="brown")
        graph.add_import(importer="brown", imported="blue.bar")

        # Long indirect import.
        graph.add_import(importer="green.foo", imported="yellow.four")
        graph.add_import(importer="yellow.four", imported="yellow.three")
        graph.add_import(importer="yellow.three", imported="yellow.two")
        graph.add_import(importer="yellow.two", imported="yellow.one")
        graph.add_import(importer="yellow.one", imported="blue.bar")

        result = graph.find_shortest_chains(importer="green", imported="blue")
        assert result == {
            ("green.foo", "blue.foo"),
            (
                "green.foo",
                "yellow.four",
                "yellow.three",
                "yellow.two",
                "yellow.one",
                "blue.bar",
            ),
        }

    @pytest.mark.parametrize("as_packages", (False, True))
    def test_doesnt_lose_import_details(self, as_packages: bool):
        # Find shortest chains uses hiding mechanisms, this checks that it doesn't
        # inadvertently delete import details for the things it hides.
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")
        graph.add_import(
            importer="green.foo",
            imported="blue.bar",
            line_contents="import blue.bar",
            line_number=5,
        )

        graph.find_shortest_chains(importer="green", imported="blue", as_packages=as_packages)

        assert graph.get_import_details(importer="green.foo", imported="blue.bar") == [
            {
                "importer": "green.foo",
                "imported": "blue.bar",
                "line_contents": "import blue.bar",
                "line_number": 5,
            }
        ]

    @pytest.mark.parametrize("as_packages", (False, True))
    def test_doesnt_change_import_count(self, as_packages: bool):
        # Find shortest chains uses hiding mechanisms, this checks that it doesn't
        # inadvertently change the import count.
        graph = ImportGraph()
        graph.add_module("green")
        graph.add_module("blue")
        graph.add_import(importer="green.foo", imported="blue.bar")
        count_prior = graph.count_imports()

        graph.find_shortest_chains(importer="green", imported="blue", as_packages=as_packages)

        assert graph.count_imports() == count_prior


@pytest.mark.parametrize(
    "importer, imported, as_packages, expected_result",
    (
        # This block: as_packages not supplied.
        ("a.two", "a.one", None, True),  # Importer directly imports imported.
        ("a.three", "a.one", None, True),  # Importer indirectly imports imported.
        # Importer does not import the imported, even indirectly.
        ("a.two.green", "a.one", None, False),
        ("b.two", "c.one", None, False),  # Importer imports the child of the imported.
        ("b.two", "b", None, False),  # Importer is child of imported (but doesn't import).
        # Importer's child imports imported's child.
        ("b.two", "a.one", None, False),
        # Importer's grandchild directly imports imported's grandchild.
        ("b", "a", None, False),
        # Importer's grandchild indirectly imports imported's grandchild.
        ("d", "a", None, False),
        # 'Weak dependency': importer's child imports module that does not import imported
        # (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        # The chains are: e.one -> b.one; b.two -> c.one.green; c.one -> a.two.
        ("e", "a", None, False),
        # This block: as_packages=False (should be identical to block above).
        ("a.two", "a.one", False, True),
        ("a.three", "a.one", False, True),
        ("a.two.green", "a.one", False, False),
        ("b.two", "c.one", False, False),
        ("b.two", "b", False, False),
        ("b.two", "a.one", False, False),
        ("b", "a", False, False),
        ("d", "a", False, False),
        ("e", "a", False, False),
        # This block: as_packages=True.
        ("a.two", "a.one", True, True),  # Importer directly imports imported.
        ("a.three", "a.one", True, True),  # Importer indirectly imports imported.
        # Importer does not import the imported, even indirectly.
        ("a.two.green", "a.one", True, False),
        # Importer imports the child of the imported (b.two -> c.one.green).
        ("b.two", "c.one", True, True),
        # Importer is child of imported (but doesn't import). This doesn't
        # make sense if as_packages is True, so it should raise an exception.
        ("b.two", "b", True, ValueError()),
        # Importer is child of imported (but doesn't import). This doesn't
        # make sense if as_packages is True, so it should raise an exception.
        ("b", "b.two", True, ValueError()),
        # Importer's child imports imported's child (b.two.green -> a.one.green).
        ("b.two", "a.one", True, True),
        # Importer's grandchild directly imports imported's grandchild
        # (b.two.green -> a.one.green).
        ("b", "a", True, True),
        # Importer's grandchild indirectly imports imported's grandchild.
        # (d.one.green -> b.two.green -> a.one.green).
        ("d", "a", True, True),
        # 'Weak dependency': importer's child imports module that does not import imported
        # (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module. We treat this as False as it's not really a dependency.
        # The chains are: e.one -> b.one; b.two -> c.one.green; c.one -> a.two.
        ("e", "a", True, False),
        # Squashed modules.
        ("a.three", "squashed", False, True),  # Direct import of squashed module.
        ("a.three", "squashed", True, True),
        ("squashed", "a.two", False, True),  # Direct import by squashed module.
        ("squashed", "a.two", True, True),
        ("squashed", "a.one", False, True),  # Indirect import by squashed module.
        ("squashed", "a.one", True, True),
        ("a", "squashed", False, False),  # Package involving squashed module.
        ("a", "squashed", True, True),  # Package involving squashed module.
    ),
)
def test_chain_exists(importer, imported, as_packages, expected_result):
    """
    Build a graph to analyse for chains. This is much easier to debug visually,
    so here is the dot syntax for the graph, which can be viewed using a dot file viewer.

        digraph {
            a;
            a_one;
            a_one_green;
            a_two -> a_one;
            a_two_green;
            a_three -> c_one;
            b;
            b_one;
            b_two -> c_one_green;
            b_two_green -> b_one;
            b_two_green -> a_one_green;
            c;
            c_one -> a_two;
            c_one_green;
            d;
            d_one;
            d_one_green -> b_two_green;
            e;
            e_one -> b_one;
            squashed -> a_two;
            a_three -> squashed;

    """
    graph = ImportGraph()
    a, a_one, a_one_green, a_two, a_two_green, a_three = (
        "a",
        "a.one",
        "a.one.green",
        "a.two",
        "a.two.green",
        "a.three",
    )
    b, b_one, b_two, b_two_green = ("b", "b.one", "b.two", "b.two.green")
    c, c_one, c_one_green = "c", "c.one", "c.one.green"
    d, d_one, d_one_green = "d", "d.one", "d.one.green"
    e, e_one = "e", "e.one"
    squashed = "squashed"

    for module_to_add in (
        a,
        a_one,
        a_one_green,
        a_two,
        a_two_green,
        a_three,
        b,
        b_one,
        b_two,
        b_two_green,
        c,
        c_one,
        c_one_green,
        d,
        d_one,
        d_one_green,
        e,
        e_one,
    ):
        graph.add_module(module_to_add)
    graph.add_module(squashed, is_squashed=True)

    for _importer, _imported in (
        (a_two, a_one),
        (c_one, a_two),
        (a_three, c_one),
        (b_two, c_one_green),
        (b_two_green, b_one),
        (b_two_green, a_one_green),
        (d_one_green, b_two_green),
        (e_one, b_one),
        (squashed, a_two),
        (a_three, squashed),
    ):
        graph.add_import(importer=_importer, imported=_imported)

    kwargs = dict(imported=imported, importer=importer)
    if as_packages is not None:
        kwargs["as_packages"] = as_packages

    if isinstance(expected_result, Exception):
        with pytest.raises(expected_result.__class__):
            graph.chain_exists(**kwargs)
    else:
        assert expected_result == graph.chain_exists(**kwargs)
