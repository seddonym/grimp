import re
from concurrent.futures.process import BrokenProcessPool

import pytest  # type: ignore
from grimp import build_graph, exceptions


def test_syntax_error_terminates_executor_pool():
    with pytest.raises(BrokenProcessPool):
        build_graph("syntaxerrorpackage", cache_dir=None)


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
