import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph



class TestFindShortestChainTurbo:
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

        assert (a, b, c) == graph.find_shortest_chain_turbo(importer=a, imported=c)

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

        assert (a, b) == graph.find_shortest_chain_turbo(importer=a, imported=b)

    def test_find_shortest_chain_returns_none_if_not_exists(self):
        graph = ImportGraph()
        a, b, c = "foo", "bar", "baz"

        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=b, imported=c)

        assert None is graph.find_shortest_chain_turbo(importer=c, imported=a)

    @pytest.mark.skip()
    def test_raises_value_error_if_importer_not_present(self):
        graph = ImportGraph()

        with pytest.raises(ValueError, match="Module foo is not present in the graph."):
            graph.find_shortest_chain_turbo(importer="foo", imported="bar")

    @pytest.mark.skip()
    def test_raises_value_error_if_imported_not_present(self):
        graph = ImportGraph()
        graph.add_module("foo")

        with pytest.raises(ValueError, match="Module bar is not present in the graph."):
            graph.find_shortest_chain_turbo(importer="foo", imported="bar")

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

        assert (a, b, c, d, e) == graph.find_shortest_chain_turbo(importer=a, imported=e)
