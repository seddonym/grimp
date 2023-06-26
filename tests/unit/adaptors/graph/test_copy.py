from copy import deepcopy

from grimp.adaptors.graph import ImportGraph


class TestCopy:
    def test_removing_import_doesnt_affect_copy(self):
        graph = ImportGraph()
        graph.add_import(
            importer="foo", imported="bar", line_number=3, line_contents="import bar"
        )
        graph.add_import(importer="bar", imported="baz")
        copied_graph = deepcopy(graph)

        graph.remove_import(importer="foo", imported="bar")

        assert copied_graph.find_shortest_chain(importer="foo", imported="baz") == (
            "foo",
            "bar",
            "baz",
        )
        assert copied_graph.get_import_details(importer="foo", imported="bar") == [
            {
                "importer": "foo",
                "imported": "bar",
                "line_number": 3,
                "line_contents": "import bar",
            }
        ]

    def test_can_mutate_import_details_externally(self):
        graph = ImportGraph()
        original_line_contents = "import bar"
        graph.add_import(
            importer="foo",
            imported="bar",
            line_number=3,
            line_contents=original_line_contents,
        )
        [details] = graph.get_import_details(importer="foo", imported="bar")
        copied_graph = deepcopy(graph)

        details["line_contents"] = "changed"
        [copied_graph_details] = copied_graph.get_import_details(
            importer="foo", imported="bar"
        )

        assert copied_graph_details["line_contents"] == original_line_contents

    def test_copies_squashed_modules(self):
        graph = ImportGraph()
        modules_to_squash = {
            "foo",
            "foo.green",
            "foo.blue",
            "foo.blue.alpha",
        }
        other_modules = {
            "bar",
            "bar.black",
            "baz",
        }
        for module in modules_to_squash | other_modules:
            graph.add_module(module)
        graph.squash_module("foo")

        copied_graph = deepcopy(graph)

        assert copied_graph.is_module_squashed("foo")

    def test_does_not_share_squashed_modules(self):
        graph = ImportGraph()
        modules_to_squash = {
            "foo",
            "foo.green",
            "foo.blue",
            "foo.blue.alpha",
        }
        other_modules = {
            "bar",
            "bar.black",
            "baz",
        }
        for module in modules_to_squash | other_modules:
            graph.add_module(module)
        copied_graph = deepcopy(graph)

        graph.squash_module("foo")

        assert not copied_graph.is_module_squashed("foo")
