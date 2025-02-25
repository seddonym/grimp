import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph
from grimp.exceptions import InvalidModuleExpression


def test_find_modules_directly_imported_by():
    graph = ImportGraph()
    a, b, c = "foo", "bar", "baz"
    d, e, f = "foo.one", "bar.one", "baz.one"

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=b, imported=e)
    graph.add_import(importer=f, imported=a)

    assert {b, c, d} == graph.find_modules_directly_imported_by("foo")


def test_find_modules_that_directly_import():
    graph = ImportGraph()
    a, b, c = "foo", "bar", "baz"
    d, e, f = "foo.one", "bar.one", "baz.one"

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=b, imported=e)
    graph.add_import(importer=f, imported=b)

    assert {a, f} == graph.find_modules_that_directly_import("bar")


@pytest.mark.parametrize(
    "importer, imported, as_packages, expected_result",
    (
        # as_packages=False:
        ("a.one.green", "a.two.green", False, True),  # Direct import.
        ("a.two.green", "a.three.blue", False, True),  # Direct import.
        ("a.one.green", "a.three.blue", False, False),  # Indirect import.
        ("a.two.green", "a.one.green", False, False),  # Reverse direct import.
        ("a.one", "a.two", False, False),  # Direct import - parents.
        ("a.two", "a.two.green", False, True),  # Direct import - parent to child.
        # as_packages=True:
        ("a.one.green", "a.two.green", True, True),  # Direct import.
        ("a.one.green", "a.three.blue", True, False),  # Indirect import.
        ("a.one", "a.two", True, True),  # Direct import - parents.
        ("a.one", "a.three", True, False),  # Indirect import - parents.
        # Direct import - importer child, imported actual.
        ("a.four.green", "a.two.green", True, True),
        # Direct import - importer actual, imported child.
        ("a.five", "a.four", True, True),
        # Direct import - importer grandchild, imported child.
        ("a.four", "a.two", True, True),
        # Exceptions - doesn't make sense to ask about direct imports within package
        # when as_packages=True.
        ("a.two", "a.two.green", True, ValueError()),
        ("a.two.green", "a.two", True, ValueError()),
    ),
)
def test_direct_import_exists(importer, imported, as_packages, expected_result):
    """
    Build a graph to analyse for chains. This is much easier to debug visually,
    so here is the dot syntax for the graph, which can be viewed using a dot file viewer.

        digraph {
            a;
            a_one;
            a_one_green;
            a_one_blue;
            a_two;
            a_two_green;
            a_two_blue;
            a_three;
            a_three_green;
            a_three_blue;
            a_four;
            a_four_green;
            a_four_green_alpha;
            a_five;

            a_one_green -> a_two_green;
            a_two -> a_two_green;
            a_two_green -> a_three_blue;
            a_five -> a_four_green_alpha;
            a_four_green_alpha -> a_two_green;
        }
    """
    graph = ImportGraph()
    all_modules = (
        a,
        a_one,
        a_one_green,
        a_one_blue,
        a_two,
        a_two_green,
        a_two_blue,
        a_three,
        a_three_green,
        a_three_blue,
        a_four,
        a_four_green,
        a_four_green_alpha,
        a_five,
    ) = (
        "a",
        "a.one",
        "a.one.green",
        "a.one.blue",
        "a.two",
        "a.two.green",
        "a.two.blue",
        "a.three",
        "a.three.green",
        "a.three.blue",
        "a.four",
        "a.four.green",
        "a.four.green.alpha",
        "a.five",
    )

    for module_to_add in all_modules:
        graph.add_module(module_to_add)

    for _importer, _imported in (
        (a_one_green, a_two_green),
        (a_two, a_two_green),
        (a_two_green, a_three_blue),
        (a_five, a_four_green_alpha),
        (a_four_green_alpha, a_two_green),
    ):
        graph.add_import(importer=_importer, imported=_imported)

    if isinstance(expected_result, Exception):
        with pytest.raises(expected_result.__class__):
            graph.direct_import_exists(
                importer=importer, imported=imported, as_packages=as_packages
            )
    else:
        assert expected_result == graph.direct_import_exists(
            importer=importer, imported=imported, as_packages=as_packages
        )


class TestCountImports:
    @pytest.mark.parametrize(
        "imports, expected_count",
        (
            ((), 0),
            ((("foo.one", "foo.two"),), 1),
            ((("foo.one", "foo.two"), ("foo.three", "foo.two")), 2),
            (
                (
                    ("foo.one", "foo.two"),
                    ("foo.three", "foo.two"),
                    (
                        "foo.three",
                        "foo.two",
                    ),  # Duplicate should not increase the number.
                ),
                2,
            ),
        ),
    )
    def test_count_imports(self, imports, expected_count):
        graph = ImportGraph()

        for importer, imported in imports:
            graph.add_import(importer=importer, imported=imported)

        assert expected_count == graph.count_imports()

    def test_count_imports_for_multiple_imports_between_same_modules(self):
        # Count imports does not actually return the number of imports, but the number of
        # dependencies between modules.
        graph = ImportGraph()

        graph.add_import(
            importer="blue",
            imported="green",
            line_contents="import green",
            line_number=1,
        )
        graph.add_import(
            importer="blue",
            imported="green",
            line_contents="from green import *",
            line_number=2,
        )

        assert graph.count_imports() == 1

    def test_count_imports_after_removals(self):
        graph = ImportGraph()

        graph.add_import(importer="blue", imported="green")
        graph.add_import(importer="blue", imported="orange")
        graph.add_import(importer="blue", imported="yellow")
        graph.add_import(importer="green", imported="orange")

        graph.remove_import(importer="blue", imported="yellow")

        assert graph.count_imports() == 3


class TestGetImportDetails:
    def test_happy_path(self):
        graph = ImportGraph()

        imports_info = [
            dict(
                importer="mypackage.foo",
                imported="mypackage.bar",
                line_number=1,
                line_contents="from . import bar",
            ),
            dict(
                importer="mypackage.foo",
                imported="mypackage.bar",
                line_number=10,
                line_contents="from .bar import a_function",
            ),
        ]
        for import_info in imports_info:
            graph.add_import(**import_info)

        assert imports_info == graph.get_import_details(
            importer="mypackage.foo", imported="mypackage.bar"
        )

    def test_returns_empty_list_when_no_import(self):
        graph = ImportGraph()

        assert [] == graph.get_import_details(importer="foo", imported="bar")

    def test_returns_empty_list_when_import_but_no_available_details(self):
        graph = ImportGraph()

        importer, imported = "foo", "bar"
        graph.add_import(importer=importer, imported=imported),

        assert [] == graph.get_import_details(importer=importer, imported=imported)

    def test_returns_only_relevant_imports(self):
        graph = ImportGraph()

        imports_info = [
            dict(
                importer="mypackage.foo",
                imported="mypackage.bar",
                line_number=1,
                line_contents="from . import bar",
            )
        ]
        graph.add_import(**imports_info[0])

        # Also add a different import in the same module.
        graph.add_import(
            importer="mypackage.foo",
            imported="mypackage.baz",
            line_number=2,
            line_contents="from . import baz",
        )

        assert imports_info == graph.get_import_details(
            importer="mypackage.foo", imported="mypackage.bar"
        )


class TestFindMatchingDirectImports:
    @pytest.mark.parametrize(
        "import_line_number,import_line_contents",
        [
            [None, None],
            [1, "..."],
        ],
    )
    def test_finds_matching_direct_imports(self, import_line_number, import_line_contents):
        graph = ImportGraph()
        # Should match
        graph.add_import(
            importer="pkg.animals.dog",
            imported="pkg.food.chicken",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )
        graph.add_import(
            importer="pkg.animals.cat",
            imported="pkg.food.fish",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )
        # Should not match: Imported does not match
        graph.add_import(
            importer="pkg.animals.dog",
            imported="pkg.colors.golden",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )
        graph.add_import(
            importer="pkg.animals.cat",
            imported="pkg.colors.ginger",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )
        # Should not match: Importer does not match
        graph.add_import(
            importer="pkg.shops.tesco",
            imported="pkg.food.chicken",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )
        graph.add_import(
            importer="pkg.shops.coop",
            imported="pkg.food.fish",
            line_number=import_line_number,
            line_contents=import_line_contents,
        )

        assert graph.find_matching_direct_imports(
            importer_expression="pkg.animals.*", imported_expression="pkg.food.*"
        ) == [
            {"importer": "pkg.animals.cat", "imported": "pkg.food.fish"},
            {"importer": "pkg.animals.dog", "imported": "pkg.food.chicken"},
        ]

    def test_deduplicates_imports(self):
        graph = ImportGraph()
        graph.add_import(
            importer="pkg.animals.dog",
            imported="pkg.colors.golden",
            line_number=1,
            line_contents="...1",
        )
        graph.add_import(
            importer="pkg.animals.dog",
            imported="pkg.colors.golden",
            line_number=2,
            line_contents="...2",
        )

        assert graph.find_matching_direct_imports(
            importer_expression="pkg.animals.*", imported_expression="pkg.colors.*"
        ) == [
            {"importer": "pkg.animals.dog", "imported": "pkg.colors.golden"},
        ]

    def test_raises_error_if_importer_expression_is_invalid(self):
        graph = ImportGraph()
        with pytest.raises(
            InvalidModuleExpression, match="foo.. is not a valid module expression."
        ):
            graph.find_matching_direct_imports(
                importer_expression="foo..", imported_expression="bar"
            )

    def test_raises_error_if_imported_expression_is_invalid(self):
        graph = ImportGraph()
        with pytest.raises(
            InvalidModuleExpression, match="bar.. is not a valid module expression."
        ):
            graph.find_matching_direct_imports(
                importer_expression="foo", imported_expression="bar.."
            )
