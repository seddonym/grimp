import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph
from grimp.exceptions import ModuleNotPresent, InvalidModuleExpression


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


def test_find_children_raises_exception_for_missing_module():
    graph = ImportGraph()
    graph.add_module("foo.a.one")

    with pytest.raises(ModuleNotPresent):
        graph.find_children("foo.a")


def test_find_children_raises_exception_for_squashed_module():
    graph = ImportGraph()
    module = "foo"

    graph.add_module(module, is_squashed=True)

    with pytest.raises(ValueError, match="Cannot find children of a squashed module."):
        graph.find_children(module)


def test_adding_same_child_module_twice_does_not_corrupt_hierarchy():
    graph = ImportGraph()
    graph.add_module("mypackage.blue")
    graph.add_module("mypackage.blue.alpha")
    graph.add_module("mypackage.blue")  # Add for second time.
    graph.add_module("mypackage.blue.beta")

    result = graph.find_children("mypackage.blue")

    assert result == {"mypackage.blue.alpha", "mypackage.blue.beta"}


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


def test_find_descendants_works_with_gaps():
    graph = ImportGraph()
    graph.add_module("mypackage.foo")
    # We do not add "mypackage.foo.blue" - there's a gap.
    graph.add_module("mypackage.foo.blue.alpha")
    graph.add_module("mypackage.foo.blue.alpha.one")
    graph.add_module("mypackage.foo.blue.alpha.two")
    graph.add_module("mypackage.foo.blue.beta.three")
    graph.add_module("mypackage.bar.green.alpha")

    result = graph.find_descendants("mypackage.foo")

    assert result == {
        "mypackage.foo.blue.alpha",
        "mypackage.foo.blue.alpha.one",
        "mypackage.foo.blue.alpha.two",
        "mypackage.foo.blue.beta.three",
    }


def test_find_descendants_works_if_modules_added_in_different_order():
    graph = ImportGraph()
    graph.add_module("mypackage.foo")
    graph.add_module("mypackage.foo.blue.alpha")
    graph.add_module("mypackage.foo.blue.alpha.one")
    graph.add_module("mypackage.bar.green.beta")
    # Add the middle item in the hierarchy last.
    graph.add_module("mypackage.foo.blue")

    result = graph.find_descendants("mypackage.foo")

    assert result == {
        "mypackage.foo.blue",
        "mypackage.foo.blue.alpha",
        "mypackage.foo.blue.alpha.one",
    }


class TestFindMatchingModules:
    @pytest.mark.parametrize(
        "expression, expected_matching_modules",
        [
            ["foo", {"foo"}],
            ["foo.*", {"foo.bar"}],
            ["foo.**", {"foo.bar", "foo.bar.baz"}],
        ],
    )
    def test_finds_matching_modules(self, expression, expected_matching_modules):
        graph = ImportGraph()
        graph.add_module("foo")
        graph.add_module("foo.bar")
        graph.add_module("foo.bar.baz")
        assert graph.find_matching_modules(expression) == expected_matching_modules

    def test_does_not_return_invisible_modules(self):
        graph = ImportGraph()
        # "foo" and "foo.bar" will be invisible modules in the graph.
        graph.add_module("foo.bar.baz")

        # "foo.bar" is not returned.
        assert graph.find_matching_modules("foo.**") == {"foo.bar.baz"}

    def test_raises_error_if_expression_is_invalid(self):
        graph = ImportGraph()
        with pytest.raises(
            InvalidModuleExpression, match="foo.. is not a valid module expression."
        ):
            graph.find_matching_modules("foo..")
