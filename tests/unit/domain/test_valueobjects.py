from grimp.domain.valueobjects import Module, DirectImport, ImportChain


class TestModule:
    def test_repr(self):
        module = Module('foo.bar')
        assert repr(module) == '<Module: foo.bar>'

    def test_equals(self):
        a = Module('foo.bar')
        b = Module('foo.bar')
        c = Module('foo.bar.baz')

        assert a == b
        assert a != c
        # Also non-Module instances should not be treated as equal.
        assert a != 'foo'

    def test_hash(self):
        a = Module('foo.bar')
        b = Module('foo.bar')
        c = Module('foo.bar.baz')

        assert hash(a) == hash(b)
        assert hash(a) != hash(c)

    def test_package_name(self):
        assert Module('foo.bar.baz').package_name == 'foo'


class TestDirectImport:
    def test_repr(self):
        import_path = DirectImport(
            importer=Module('foo'), imported=Module('bar'),
            line_number=10, line_contents='import bar',
        )
        assert repr(import_path) == '<DirectImport: foo -> bar (l. 10)>'

    def test_equals(self):
        a = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        b = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        c = DirectImport(
            importer=Module('foo'),
            imported=Module('baz'),
            line_number=10,
            line_contents='import bar',
        )
        d = DirectImport(
            importer=Module('foobar'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        e = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=11,
            line_contents='import bar',
        )
        f = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='from . import bar',
        )

        assert a == b
        assert a != c
        assert a != d
        assert a != e
        assert a != f
        # Also non-DirectImport instances should not be treated as equal.
        assert a != 'foo'

    def test_hash(self):
        a = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        b = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        c = DirectImport(
            importer=Module('foo'),
            imported=Module('baz'),
            line_number=10,
            line_contents='import bar',
        )
        d = DirectImport(
            importer=Module('foobar'),
            imported=Module('bar'),
            line_number=10,
            line_contents='import bar',
        )
        e = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=11,
            line_contents='import bar',
        )
        f = DirectImport(
            importer=Module('foo'),
            imported=Module('bar'),
            line_number=10,
            line_contents='from . import bar',
        )

        assert hash(a) == hash(b)
        assert hash(a) != hash(c)
        assert hash(a) != hash(d)
        assert hash(a) != hash(e)
        assert hash(a) != hash(f)


class TestImportChain:
    def test_repr(self):
        import_path = ImportChain(
            Module('one'),
            Module('two'),
            Module('three'),
        )
        assert repr(import_path) == '<ImportChain: three -> two -> one>'

    def test_equals(self):
        a = ImportChain(
            Module('one'),
            Module('two'),
            Module('three'),
        )
        b = ImportChain(
            Module('one'),
            Module('two'),
            Module('three'),
        )
        c = ImportChain(
            Module('one'),
            Module('three'),
            Module('two'),
        )

        assert a == b
        assert a != c
        # Also non-ImportChain instances should not be treated as equal.
        assert a != 'foo'

    def test_hash(self):
        a = ImportChain(
            Module('one'),
            Module('two'),
            Module('three'),
        )
        b = ImportChain(
            Module('one'),
            Module('two'),
            Module('three'),
        )
        c = ImportChain(
            Module('one'),
            Module('three'),
            Module('two'),
        )

        assert hash(a) == hash(b)
        assert hash(a) != hash(c)
