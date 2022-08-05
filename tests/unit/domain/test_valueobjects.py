import pytest

from grimp.domain.valueobjects import DirectImport, Module


class TestModule:
    def test_repr(self):
        module = Module("foo.bar")
        assert repr(module) == "<Module: foo.bar>"

    def test_equals(self):
        a = Module("foo.bar")
        b = Module("foo.bar")
        c = Module("foo.bar.baz")

        assert a == b
        assert a != c
        # Also non-Module instances should not be treated as equal.
        assert a != "foo"

    def test_hash(self):
        a = Module("foo.bar")
        b = Module("foo.bar")
        c = Module("foo.bar.baz")

        assert hash(a) == hash(b)
        assert hash(a) != hash(c)

    @pytest.mark.parametrize(
        "module, expected",
        (
            (Module("foo.bar.baz"), "foo"),
            (Module("foo.bar.baz", top_level_package="foo.bar"), "foo.bar"),
        ),
    )
    def test_package_name(self, module, expected):
        assert module.package_name == expected

    @pytest.mark.parametrize(
        "module, expected",
        (
            (Module("foo"), ValueError("Module has no parent.")),
            (Module("foo.bar"), Module("foo")),
            (Module("foo.bar.baz"), Module("foo.bar")),
            (
                Module("foo.bar", top_level_package="foo.bar"),
                ValueError("Module has no parent."),
            ),
            (Module("foo.bar.baz", top_level_package="foo.bar"), Module("foo.bar")),
            (
                Module("foo.bar.baz.foobar", top_level_package="foo.bar"),
                Module("foo.bar.baz"),
            ),
        ),
    )
    def test_parent(self, module, expected):
        if isinstance(expected, Exception):
            with pytest.raises(expected.__class__, match=str(expected)):
                module.parent
        else:
            assert module.parent == expected


class TestDirectImport:
    def test_repr(self):
        import_path = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        assert repr(import_path) == "<DirectImport: foo -> bar (l. 10)>"

    def test_equals(self):
        a = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        b = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        c = DirectImport(
            importer=Module("foo"),
            imported=Module("baz"),
            line_number=10,
            line_contents="import bar",
        )
        d = DirectImport(
            importer=Module("foobar"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        e = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=11,
            line_contents="import bar",
        )
        f = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="from . import bar",
        )

        assert a == b
        assert a != c
        assert a != d
        assert a != e
        assert a != f
        # Also non-DirectImport instances should not be treated as equal.
        assert a != "foo"

    def test_hash(self):
        a = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        b = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        c = DirectImport(
            importer=Module("foo"),
            imported=Module("baz"),
            line_number=10,
            line_contents="import bar",
        )
        d = DirectImport(
            importer=Module("foobar"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        e = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=11,
            line_contents="import bar",
        )
        f = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="from . import bar",
        )

        assert hash(a) == hash(b)
        assert hash(a) != hash(c)
        assert hash(a) != hash(d)
        assert hash(a) != hash(e)
        assert hash(a) != hash(f)
