import pytest

import grimp


def test_build_graph_of_non_ascii_source():
    """
    Tests we can cope with non ascii Python source files.
    """
    graph = grimp.build_graph("encodingpackage", cache_dir=None)

    result = graph.get_import_details(
        importer="encodingpackage.importer", imported="encodingpackage.imported"
    )

    assert [
        {
            "importer": "encodingpackage.importer",
            "imported": "encodingpackage.imported",
            "is_lazy": False,
            "line_number": 1,
            "line_contents": "from .imported import π",
        },
    ] == result


def test_build_graph_of_non_utf8_source():
    """
    Tests we can cope with non UTF-8 Python source files.
    """
    graph = grimp.build_graph("encodingpackage", cache_dir=None)

    result = graph.get_import_details(
        importer="encodingpackage.shift_jis_importer", imported="encodingpackage.imported"
    )

    assert [
        {
            "importer": "encodingpackage.shift_jis_importer",
            "imported": "encodingpackage.imported",
            "is_lazy": False,
            "line_number": 3,
            "line_contents": "from .imported import π",
        },
    ] == result


@pytest.mark.parametrize(
    "declared_encoding, codec, imported_name",
    (
        # Python special cases these spellings of UTF-8 and Latin-1.
        ("utf_8", "utf-8", "π"),
        ("UTF-8-sig", "utf-8", "π"),
        ("latin-1", "latin-1", "jalapeño"),
        ("latin_1", "latin-1", "jalapeño"),
        ("iso_8859_1", "latin-1", "jalapeño"),
        # Python also accepts underscores in place of hyphens in other encoding names.
        ("euc_jp", "euc_jp", "ラーメン"),
        ("iso8859_15", "iso8859_15", "jalapeño"),
    ),
)
def test_build_graph_of_source_declaring_encoding_with_python_specific_name(
    tmp_path, monkeypatch, declared_encoding, codec, imported_name
):
    """
    Tests we can cope with source files that declare their encoding using a name that Python
    accepts, but that isn't a WHATWG encoding label.
    """
    package_directory = tmp_path / "declaredencodingpackage"
    package_directory.mkdir()
    (package_directory / "__init__.py").write_text("")
    (package_directory / "imported.py").write_text("")
    (package_directory / "importer.py").write_bytes(
        f"# -*- coding: {declared_encoding} -*-\nfrom .imported import {imported_name}\n".encode(
            codec
        )
    )
    monkeypatch.syspath_prepend(str(tmp_path))

    graph = grimp.build_graph("declaredencodingpackage", cache_dir=None)

    result = graph.get_import_details(
        importer="declaredencodingpackage.importer", imported="declaredencodingpackage.imported"
    )

    assert [
        {
            "importer": "declaredencodingpackage.importer",
            "imported": "declaredencodingpackage.imported",
            "is_lazy": False,
            "line_number": 2,
            "line_contents": f"from .imported import {imported_name}",
        },
    ] == result
