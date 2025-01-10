import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph
from grimp.exceptions import ModuleNotPresent


class TestAddModule:
    def test_add_module(self):
        graph = ImportGraph()
        module = "foo"

        graph.add_module(module)

        assert graph.modules == {module}

    def test_add_module_does_not_add_ancestors_too(self):
        graph = ImportGraph()
        module = "mypackage.foo.bar"

        graph.add_module(module)

        assert graph.modules == {"mypackage.foo.bar"}


class TestRemoveModule:
    def test_removes_module_removes_import_details_for_imported(self):
        graph = ImportGraph()
        a, b, c = "mypackage.blue", "mypackage.green", "mypackage.yellow"

        graph.add_import(
            importer=a,
            imported=b,
            line_contents="import mypackage.green",
            line_number=1,
        )
        graph.add_import(
            importer=a,
            imported=c,
            line_contents="import mypackage.yellow",
            line_number=2,
        )

        graph.remove_module(b)
        assert graph.get_import_details(importer=a, imported=b) == []
        assert graph.get_import_details(importer=a, imported=c) == [
            {
                "importer": a,
                "imported": c,
                "line_contents": "import mypackage.yellow",
                "line_number": 2,
            }
        ]

    def test_removes_module_from_modules(self):
        graph = ImportGraph()
        a, b = "mypackage.blue", "mypackage.green"

        graph.add_module(a)
        graph.add_module(b)
        graph.add_import(importer=a, imported=b)

        graph.remove_module(b)
        assert graph.modules == {a}

    def test_removes_module_removes_import_details_for_importer(self):
        graph = ImportGraph()
        a, b, c = "mypackage.blue", "mypackage.green", "mypackage.yellow"

        graph.add_import(
            importer=b,
            imported=a,
            line_contents="import mypackage.blue",
            line_number=1,
        )
        graph.add_import(
            importer=a,
            imported=c,
            line_contents="import mypackage.yellow",
            line_number=2,
        )

        graph.remove_module(b)
        assert graph.get_import_details(importer=b, imported=a) == []
        assert graph.get_import_details(importer=a, imported=c) == [
            {
                "importer": a,
                "imported": c,
                "line_contents": "import mypackage.yellow",
                "line_number": 2,
            }
        ]

    def test_removing_non_existent_module_doesnt_error(self):
        graph = ImportGraph()
        a, b = "mypackage.blue", "mypackage.green"

        graph.add_module(a)
        graph.add_module(b)
        graph.add_import(importer=a, imported=b)

        graph.remove_module("mypackage.yellow")


class TestAddSquashedModule:
    def test_can_repeatedly_add_same_squashed_module(self):
        graph = ImportGraph()
        module = "foo"

        graph.add_module(module, is_squashed=True)
        graph.add_module(module, is_squashed=True)

        assert graph.modules == {module}

    def test_cannot_add_squashed_module_if_already_same_unsquashed_module(self):
        graph = ImportGraph()
        module = "foo"

        graph.add_module(module)

        with pytest.raises(
            ValueError,
            match=(
                "Cannot add a squashed module when it is already present in the graph as an "
                "unsquashed module, or vice versa."
            ),
        ):
            graph.add_module(module, is_squashed=True)

    def test_cannot_add_unsquashed_module_if_already_same_squashed_module(self):
        graph = ImportGraph()
        module = "foo"

        graph.add_module(module, is_squashed=True)

        with pytest.raises(
            ValueError,
            match=(
                "Cannot add a squashed module when it is already present in the graph as an "
                "unsquashed module, or vice versa."
            ),
        ):
            graph.add_module(module)

    @pytest.mark.parametrize("module_name", ("mypackage.foo.one", "mypackage.foo.one.alpha"))
    def test_cannot_add_descendant_of_squashed_module(self, module_name):
        graph = ImportGraph()

        graph.add_module("mypackage.foo", is_squashed=True)

        with pytest.raises(
            ValueError, match="Module is a descendant of squashed module mypackage.foo."
        ):
            graph.add_module(module_name)


@pytest.mark.parametrize("add_module", (True, False))
def test_add_import(add_module):
    graph = ImportGraph()
    a, b = "foo", "bar"

    # Adding the module should make no difference to the result.
    if add_module:
        graph.add_module(a)

    graph.add_import(importer=a, imported=b)

    assert {a, b} == graph.modules
    assert {b} == graph.find_modules_directly_imported_by(a)
    assert set() == graph.find_modules_directly_imported_by(b)


class TestRemoveImport:
    def test_removes_from_modules(self):
        graph = ImportGraph()
        a, b, c = "foo", "bar", "baz"
        graph.add_import(importer=a, imported=b)
        graph.add_import(importer=a, imported=c)

        graph.remove_import(importer=a, imported=b)

        assert {a, b, c} == graph.modules
        assert {c} == graph.find_modules_directly_imported_by(a)

    def test_removes_from_import_details(self):
        graph = ImportGraph()
        a, b, c = "mypackage.blue", "mypackage.green", "mypackage.yellow"

        graph.add_import(
            importer=a,
            imported=b,
            line_contents="import mypackage.green",
            line_number=1,
        )
        graph.add_import(
            importer=a,
            imported=c,
            line_contents="import mypackage.yellow",
            line_number=2,
        )

        graph.remove_import(importer=a, imported=b)

        assert graph.get_import_details(importer=a, imported=b) == []
        assert graph.get_import_details(importer=a, imported=c) == [
            {
                "importer": a,
                "imported": c,
                "line_contents": "import mypackage.yellow",
                "line_number": 2,
            }
        ]


class TestSquashModule:
    def test_marks_module_as_squashed(self):
        graph = ImportGraph()
        modules_to_squash = {
            "foo",
            "foo.green",
        }
        for module in modules_to_squash:
            graph.add_module(module)

        graph.squash_module("foo")

        assert graph.is_module_squashed("foo")

    def test_updates_modules_in_graph(self):
        graph = ImportGraph()
        modules_to_squash = {
            "foo",
            "foo.green",
            "foo.blue",
            "foo.blue.alpha",
        }
        other_modules = {
            "bar",
            "bar.black",
            "baz",
        }
        for module in modules_to_squash | other_modules:
            graph.add_module(module)

        graph.squash_module("foo")

        assert {"foo"} | other_modules == graph.modules

    def test_keeps_import_from_squashed_root(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        graph.add_import(importer="foo", imported="bar.blue")

        graph.squash_module("foo")

        assert graph.direct_import_exists(importer="foo", imported="bar.blue")

    def test_keeps_import_of_squashed_root(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        graph.add_import(importer="bar.blue", imported="foo")

        graph.squash_module("foo")

        assert graph.direct_import_exists(importer="bar.blue", imported="foo")

    def test_contracts_import_from_descendant(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        graph.add_import(importer="foo.green", imported="bar.blue")

        graph.squash_module("foo")

        assert graph.direct_import_exists(importer="foo", imported="bar.blue")

    def test_contracts_import_to_descendant(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        graph.add_import(importer="bar.blue", imported="foo.green")

        graph.squash_module("foo")

        assert graph.direct_import_exists(importer="bar.blue", imported="foo")

    def test_doesnt_error_if_imports_within_module(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "foo.blue",
        ]:
            graph.add_module(module)
        graph.add_import(importer="foo.blue", imported="foo.green")

        graph.squash_module("foo")

    def test_import_details_from_squashed_root_are_preserved(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        import_details = dict(
            importer="foo",
            imported="bar.blue",
            line_number=1,
            line_contents="from . import bar",
        )
        graph.add_import(**import_details)

        graph.squash_module("foo")

        assert [import_details] == graph.get_import_details(importer="foo", imported="bar.blue")

    def test_import_details_to_squashed_root_are_preserved(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)
        import_details = dict(
            importer="bar.blue",
            imported="foo",
            line_number=1,
            line_contents="from . import foo",
        )
        graph.add_import(**import_details)

        graph.squash_module("foo")

        assert [import_details] == graph.get_import_details(importer="bar.blue", imported="foo")

    def test_original_import_details_from_descendant_are_lost(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)

        graph.add_import(
            importer="foo.green",
            imported="bar.blue",
            line_number=1,
            line_contents="from . import bar.blue",
        )

        graph.squash_module("foo")

        assert [] == graph.get_import_details(importer="foo.green", imported="bar.blue")

    def test_original_import_details_to_descendant_are_lost(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)

        graph.add_import(
            importer="bar.blue",
            imported="foo.green",
            line_number=1,
            line_contents="from foo import green",
        )

        graph.squash_module("foo")

        assert [] == graph.get_import_details(importer="bar.blue", imported="foo.green")

    def test_import_details_from_descendant_are_lost(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)

        graph.add_import(
            importer="foo.green",
            imported="bar.blue",
            line_number=1,
            line_contents="from . import bar.blue",
        )

        graph.squash_module("foo")

        assert [] == graph.get_import_details(importer="foo", imported="bar.blue")

    def test_import_details_to_descendant_are_lost(self):
        graph = ImportGraph()
        for module in [
            "foo",
            "foo.green",
            "bar.blue",
        ]:
            graph.add_module(module)

        graph.add_import(
            importer="bar.blue",
            imported="foo.green",
            line_number=1,
            line_contents="from foo import green",
        )

        graph.squash_module("foo")

        assert [] == graph.get_import_details(importer="bar.blue", imported="foo")

    def test_does_nothing_if_module_is_already_squashed(self):
        graph = ImportGraph()
        graph.add_module("foo", is_squashed=True)
        graph.add_import(importer="foo", imported="bar")

        graph.squash_module("foo")

        assert graph.direct_import_exists(importer="foo", imported="bar")

    def test_raises_module_not_present_if_no_module(self):
        graph = ImportGraph()

        with pytest.raises(ModuleNotPresent):
            graph.squash_module("foo")
