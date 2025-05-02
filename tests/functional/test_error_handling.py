import re

import pytest  # type: ignore

from grimp import build_graph, exceptions


def test_does_not_raise_exception_if_encounters_syntax_error():
    """
    We don't make any promises about what to do if there's a syntax error,
    but the parser isn't a complete implementation of the Python parser, so
    it may just ignore it - finding other imports, as in this case.
    """
    graph = build_graph("syntaxerrorpackage", cache_dir=None)

    assert graph.find_modules_directly_imported_by("syntaxerrorpackage.foo.one") == {
        "syntaxerrorpackage.foo.two"
    }


def test_missing_root_init_file():
    with pytest.raises(
        exceptions.NamespacePackageEncountered,
        match=re.escape(
            "Package 'missingrootinitpackage' is a namespace package (see PEP 420). Try specifying "
            "the portion name instead. If you are not intentionally "
            "using namespace packages, adding an __init__.py file should fix the problem."
        ),
    ):
        build_graph("missingrootinitpackage", cache_dir=None)
