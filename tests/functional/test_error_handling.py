import os

import pytest  # type: ignore
from grimp import build_graph, exceptions


def test_syntax_error_includes_module():
    dirname = os.path.dirname(__file__)
    filename = os.path.abspath(
        os.path.join(dirname, "..", "assets", "syntaxerrorpackage", "foo", "one.py")
    )

    with pytest.raises(exceptions.SourceSyntaxError) as excinfo:
        build_graph("syntaxerrorpackage")

    expected_exception = exceptions.SourceSyntaxError(
        filename=filename, lineno=5, text="fromb . import two\n"
    )
    assert expected_exception == excinfo.value


@pytest.mark.xfail
def test_missing_root_init_file():
    with pytest.raises(
        exceptions.NamespacePackageEncountered,
        match=(
            r"Package missingrootinitpackage appears to be a 'namespace package' (see PEP 420),"
            r"which is not currently supported. If this is not deliberate, adding an __init__.py"
            r"file should fix the problem."
        ),
    ):
        build_graph("missingrootinitpackage")
