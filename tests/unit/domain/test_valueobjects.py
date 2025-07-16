import pytest  # type: ignore

from grimp.domain.valueobjects import DirectImport, Module, Layer


class TestModule:
    def test_str(self):
        module = Module("foo.bar")
        assert str(module) == "foo.bar"

    @pytest.mark.parametrize(
        "module, expected",
        (
            (Module("foo"), ValueError("Module has no parent.")),
            (Module("foo.bar"), Module("foo")),
            (Module("foo.bar.baz"), Module("foo.bar")),
        ),
    )
    def test_parent(self, module, expected):
        if isinstance(expected, Exception):
            with pytest.raises(expected.__class__, match=str(expected)):
                module.parent
        else:
            assert module.parent == expected


class TestDirectImport:
    def test_str(self):
        import_path = DirectImport(
            importer=Module("foo"),
            imported=Module("bar"),
            line_number=10,
            line_contents="import bar",
        )
        assert str(import_path) == "foo -> bar (l. 10)"


class TestLayer:
    def test_str(self):
        layer = Layer("foo", "bar", independent=True, closed=False)
        assert str(layer) == "['bar', 'foo'], independent=True, closed=False"
