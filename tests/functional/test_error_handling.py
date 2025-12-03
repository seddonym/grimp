import os

import pytest  # type: ignore

from grimp import build_graph, exceptions


def test_syntax_error_includes_module():
    dirname = os.path.dirname(__file__)
    filename = os.path.abspath(
        os.path.join(dirname, "..", "assets", "syntaxerrorpackage", "foo", "one.py")
    )

    with pytest.raises(exceptions.SourceSyntaxError) as excinfo:
        build_graph("syntaxerrorpackage", cache_dir=None)

    expected_exception = exceptions.SourceSyntaxError(
        filename=filename, lineno=5, text="fromb . import two"
    )
    assert expected_exception == excinfo.value
