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
            "line_number": 1,
            "line_contents": "from .imported import π",
        },
    ] == result
