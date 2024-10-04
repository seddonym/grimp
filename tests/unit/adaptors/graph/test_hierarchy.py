import pytest  # type: ignore

from grimp.adaptors.rustgraph import ImportGraph


@pytest.mark.parametrize(
    "module, expected_result",
    (("foo", {"foo.a", "foo.b", "foo.c"}), ("foo.a", {"foo.a.one"}), ("foo.c", set())),
)
def test_find_children(module, expected_result):
    graph = ImportGraph()
    foo, bar = "foo", "bar"
    a, b, c = "foo.a", "foo.b", "foo.c"
    d, e, f = "foo.a.one", "foo.b.one", "bar.g"
    for module_to_add in (foo, bar, a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_children(module)


def test_find_children_raises_exception_for_squashed_module():
    graph = ImportGraph()
    module = "foo"

    graph.add_module(module, is_squashed=True)

    with pytest.raises(ValueError, match="Cannot find children of a squashed module."):
        graph.find_children(module)


@pytest.mark.parametrize(
    "module, expected_result",
    (
        ("foo", {"foo.a", "foo.b", "foo.c", "foo.a.one", "foo.b.one"}),
        ("foo.a", {"foo.a.one"}),
        ("foo.c", set()),
    ),
)
def test_find_descendants(module, expected_result):
    graph = ImportGraph()
    foo, bar = "foo", "bar"
    a, b, c = "foo.a", "foo.b", "foo.c"
    d, e, f = "foo.a.one", "foo.b.one", "bar.g"

    for module_to_add in (foo, bar, a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_descendants(module)


def test_find_descendants_raises_exception_for_squashed_module():
    graph = ImportGraph()
    module = "foo"

    graph.add_module(module, is_squashed=True)

    with pytest.raises(ValueError, match="Cannot find descendants of a squashed module."):
        graph.find_descendants(module)
