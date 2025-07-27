from grimp.adaptors.graph import ImportGraph


class TestFindShortestCycle:
    def test_finds_shortest_cycle_when_exists(self):
        graph = ImportGraph()
        # Shortest cycle
        graph.add_import(importer="foo", imported="bar")
        graph.add_import(importer="bar", imported="baz")
        graph.add_import(importer="baz", imported="foo")
        # Longer cycle
        graph.add_import(importer="foo", imported="x")
        graph.add_import(importer="x", imported="y")
        graph.add_import(importer="y", imported="z")
        graph.add_import(importer="z", imported="foo")

        assert graph.find_shortest_cycle("foo") == ("foo", "bar", "baz", "foo")

        graph.remove_import(importer="baz", imported="foo")

        assert graph.find_shortest_cycle("foo") == ("foo", "x", "y", "z", "foo")

    def test_returns_none_if_no_cycle_exists(self):
        graph = ImportGraph()
        # Shortest cycle
        graph.add_import(importer="foo", imported="bar")
        graph.add_import(importer="bar", imported="baz")
        # graph.add_import(importer="baz", imported="foo")  # This import is missing -> No cycle.

        assert graph.find_shortest_cycle("foo") is None

    def test_ignores_internal_imports_when_as_package_is_true(self):
        graph = ImportGraph()
        graph.add_module("colors")
        graph.add_import(importer="colors.red", imported="colors.blue")
        graph.add_import(importer="colors.blue", imported="colors.red")
        graph.add_import(importer="colors.red", imported="x")
        graph.add_import(importer="x", imported="y")
        graph.add_import(importer="y", imported="z")
        graph.add_import(importer="z", imported="colors.blue")

        assert graph.find_shortest_cycle("colors", as_package=True) == (
            "colors.red",
            "x",
            "y",
            "z",
            "colors.blue",
        )
