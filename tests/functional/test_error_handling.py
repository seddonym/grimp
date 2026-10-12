import re
from pathlib import Path

import pytest

from grimp import build_graph, exceptions


def test_syntax_error_includes_module():
    filename = str(
        (
            Path(__file__).parent.parent / "assets" / "syntaxerrorpackage" / "foo" / "one.py"
        ).resolve()
    )

    with pytest.raises(exceptions.SourceSyntaxError) as excinfo:
        build_graph("syntaxerrorpackage", cache_dir=None)

    expected_exception = exceptions.SourceSyntaxError(
        filename=filename, lineno=5, text="fromb . import two"
    )
    assert expected_exception == excinfo.value


@pytest.mark.parametrize(
    "contents, expected_problem",
    (
        pytest.param(b"x = '\xff'\n", "as UTF-8", id="invalid-utf-8"),
        pytest.param(
            b"# -*- coding: euc-jp -*-\nx = '\xff\xff'\n",
            "with encoding 'euc-jp'",
            id="invalid-for-declared-encoding",
        ),
        pytest.param(
            b"# -*- coding: nonexistent -*-\n",
            "(unknown encoding 'nonexistent')",
            id="unknown-encoding",
        ),
    ),
)
def test_undecodable_source_raises_unicode_error_including_filename(
    tmp_path, monkeypatch, contents, expected_problem
):
    package_directory = tmp_path / "undecodablepackage"
    package_directory.mkdir()
    (package_directory / "__init__.py").write_text("")
    module_filename = package_directory / "undecodable.py"
    module_filename.write_bytes(contents)
    monkeypatch.syspath_prepend(str(tmp_path))

    with pytest.raises(
        UnicodeError,
        match=re.escape(f"Failed to decode file {module_filename} {expected_problem}"),
    ):
        build_graph("undecodablepackage", cache_dir=None)
