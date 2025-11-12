def test_build_graph_of_non_ascii_source(build_graph):
    """
    Tests we can cope with non ascii Python source files.
    """
    graph = build_graph("encodingpackage", cache_dir=None)

    result = graph.get_import_details(
        importer="encodingpackage.importer", imported="encodingpackage.imported"
    )

    assert [
        {
            "importer": "encodingpackage.importer",
            "imported": "encodingpackage.imported",
            "line_number": 1,
            "line_contents": "from .imported import π",
        },
    ] == result


def test_build_graph_of_non_utf8_source(build_graph):
    """
    Tests we can cope with non UTF-8 Python source files.
    """
    graph = build_graph("encodingpackage", cache_dir=None)

    result = graph.get_import_details(
        importer="encodingpackage.shift_jis_importer", imported="encodingpackage.imported"
    )

    assert [
        {
            "importer": "encodingpackage.shift_jis_importer",
            "imported": "encodingpackage.imported",
            "line_number": 3,
            "line_contents": "from .imported import π",
        },
    ] == result
