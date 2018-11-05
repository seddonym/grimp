from grimp.domain.valueobjects import Module, DirectImport

from tests.adaptors.importscanner import BaseFakeImportScanner
from tests.adaptors.filesystem import FakeFileSystem


def test_returns_imports():
    a = Module('foo')
    b = Module('bar')
    c = Module('baz')

    class FakeImportScanner(BaseFakeImportScanner):
        import_map = {
            a: {b, c},
        }

    scanner = FakeImportScanner(modules=set(),
                                package_directory='',
                                file_system=FakeFileSystem())

    result = scanner.scan_for_imports(Module('foo'))
    assert set(result) == {
        DirectImport(importer=a, imported=b),
        DirectImport(importer=a, imported=c),
    }


def test_when_no_imports_returns_empty_set():
    a = Module('foo')
    b = Module('bar')
    c = Module('baz')

    class FakeImportScanner(BaseFakeImportScanner):
        import_map = {
            a: {b, c},
        }

    scanner = FakeImportScanner(modules=set(),
                                package_directory='',
                                file_system=FakeFileSystem())

    result = scanner.scan_for_imports(Module('bar'))
    assert set(result) == set()
