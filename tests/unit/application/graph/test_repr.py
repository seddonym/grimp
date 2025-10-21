import re

from grimp.application.graph import ImportGraph


class TestRepr:
    """
    Because sets are unordered, it's difficult to test the repr in a deterministic way,
    so we use regular expressions instead.
    """

    def test_no_modules(self):
        assert "<ImportGraph: empty>" == repr(ImportGraph())

    def test_untruncated(self):
        modules = ["one", "two", "three", "four", "five"]
        graph = ImportGraph()
        for module in modules:
            graph.add_module(module)

        # Assert something in the form <ImportGraph: 'one', 'two', 'three', 'four', 'five'>
        # (but not necessarily in that order).
        re_part = "(" + "|".join(modules) + ")"
        assert re.match(
            f"<ImportGraph: '{re_part}', '{re_part}', '{re_part}', '{re_part}', '{re_part}'>",
            repr(graph),
        )

    def test_truncated(self):
        modules = ["one", "two", "three", "four", "five", "six"]
        graph = ImportGraph()
        for module in modules:
            graph.add_module(module)

        re_part = "(" + "|".join(modules) + ")"
        assert re.match(
            f"<ImportGraph: '{re_part}', '{re_part}', '{re_part}', '{re_part}', '{re_part}', ...>",
            repr(graph),
        )
