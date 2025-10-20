from grimp.application.graph import ImportGraph
import pytest


class TestNominateCycleBreakers:
    def test_empty_graph(self):
        graph = ImportGraph()
        graph.add_module("pkg")

        result = graph.nominate_cycle_breakers("pkg")

        assert result == set()

    @pytest.mark.parametrize(
        "module",
        (
            "pkg",
            "pkg.foo",
            "pkg.foo.blue",
        ),
    )
    def test_graph_with_no_imports(self, module: str):
        graph = self._build_graph_with_no_imports()

        result = graph.nominate_cycle_breakers(module)

        assert result == set()

    @pytest.mark.parametrize(
        "module",
        (
            "pkg",
            "pkg.bar",
            "pkg.foo.blue",
            "pkg.foo.green",  # Leaf package.
        ),
    )
    def test_acyclic_graph(self, module: str):
        graph = self._build_acyclic_graph()

        result = graph.nominate_cycle_breakers(module)

        assert result == set()

    def test_one_breaker(self):
        graph = self._build_acyclic_graph()
        importer, imported = "pkg.bar.red.four", "pkg.foo.blue.two"
        graph.add_import(importer=importer, imported=imported)
        result = graph.nominate_cycle_breakers("pkg")

        assert result == {(importer, imported)}

    def test_three_breakers(self):
        graph = self._build_acyclic_graph()
        imports = {
            ("pkg.bar.red.four", "pkg.foo.blue.two"),
            ("pkg.bar.yellow", "pkg.foo.blue.three"),
            ("pkg.bar", "pkg.foo.blue.three"),
        }
        for importer, imported in imports:
            graph.add_import(importer=importer, imported=imported)

        result = graph.nominate_cycle_breakers("pkg")

        assert result == imports

    def test_nominated_based_on_dependencies_rather_than_imports(self):
        graph = self._build_acyclic_graph()
        # Add lots of imports from a single module - this will be treated as
        # a single dependency.
        importer, imported = "pkg.bar.red.four", "pkg.foo.blue.two"
        for i in range(1, 30):
            graph.add_import(
                importer=importer, imported=imported, line_number=i, line_contents="-"
            )

        graph.add_import(importer=importer, imported=imported)

        result = graph.nominate_cycle_breakers("pkg")

        assert result == {(importer, imported)}

    def test_imports_between_passed_package_and_children_are_disregarded(self):
        graph = self._build_acyclic_graph()
        parent, child = "pkg.foo.blue", "pkg.foo"
        graph.add_import(importer=parent, imported=child)
        graph.add_import(importer=child, imported=parent)

        result = graph.nominate_cycle_breakers(parent)

        assert result == set()

    def test_on_child_of_root(self):
        graph = self._build_acyclic_graph()
        imports = {
            ("pkg.bar.red.five", "pkg.bar.yellow.eight"),
            ("pkg.bar.red", "pkg.bar.yellow"),
        }
        for importer, imported in imports:
            graph.add_import(importer=importer, imported=imported)

        result = graph.nominate_cycle_breakers("pkg.bar")

        assert result == imports

    def test_on_grandchild_of_root(self):
        graph = self._build_acyclic_graph()
        imports = {
            ("pkg.bar.orange.ten.gamma", "pkg.bar.orange.nine.alpha"),
            ("pkg.bar.orange.ten", "pkg.bar.orange.nine.alpha"),
        }
        for importer, imported in imports:
            graph.add_import(importer=importer, imported=imported)

        result = graph.nominate_cycle_breakers("pkg.bar.orange")

        assert result == imports

    def test_on_package_with_one_child(self):
        graph = self._build_acyclic_graph()
        graph.add_module("pkg.bar.orange.ten.gamma.onechild")

        result = graph.nominate_cycle_breakers("pkg.bar.orange.ten.gamma")

        assert result == set()

    def _build_graph_with_no_imports(self) -> ImportGraph:
        graph = ImportGraph()
        for module in (
            "pkg",
            "pkg.foo",
            "pkg.foo.blue",
            "pkg.foo.blue.one",
            "pkg.foo.blue.two",
            "pkg.foo.green",
            "pkg.bar",
            "pkg.bar.red",
            "pkg.bar.red.three",
            "pkg.bar.red.four",
            "pkg.bar.red.five",
            "pkg.bar.red.six",
            "pkg.bar.red.seven",
            "pkg.bar.yellow",
            "pkg.bar.yellow.eight",
            "pkg.bar.orange",
            "pkg.bar.orange.nine",
            "pkg.bar.orange.nine.alpha",
            "pkg.bar.orange.nine.beta",
            "pkg.bar.orange.ten",
            "pkg.bar.orange.ten.gamma",
            "pkg.bar.orange.ten.delta",
        ):
            graph.add_module(module)
        return graph

    def _build_acyclic_graph(self) -> ImportGraph:
        graph = self._build_graph_with_no_imports()
        # Add imports that make:
        #   pkg.foo -> pkg.bar
        #   pkg.bar.yellow -> pkg.foo.red
        #   pkg.bar.orange.nine -> pkg.bar.orange.ten
        for importer, imported in (
            ("pkg.foo", "pkg.bar.red"),
            ("pkg.foo.green", "pkg.bar.yellow"),
            ("pkg.foo.blue.two", "pkg.bar.red.three"),
            ("pkg.foo.blue.two", "pkg.bar.red.four"),
            ("pkg.foo.blue.two", "pkg.bar.red.five"),
            ("pkg.foo.blue.two", "pkg.bar.red.six"),
            ("pkg.foo.blue.two", "pkg.bar.red.seven"),
            ("pkg.bar.yellow", "pkg.bar.red"),
            ("pkg.bar.yellow.eight", "pkg.bar.red.three"),
            ("pkg.bar.yellow.eight", "pkg.bar.red.four"),
            ("pkg.bar.yellow.eight", "pkg.bar.red.five"),
            ("pkg.bar.orange.nine", "pkg.bar.orange.ten.gamma"),
            ("pkg.bar.orange.nine.alpha", "pkg.bar.orange.ten.gamma"),
            ("pkg.bar.orange.nine.beta", "pkg.bar.orange.ten.delta"),
        ):
            graph.add_import(importer=importer, imported=imported)
        return graph
