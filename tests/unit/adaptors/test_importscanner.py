import pytest

from grimp.adaptors.importscanner import ImportScanner
from grimp.domain.valueobjects import DirectImport, Module

from tests.adaptors.filesystem import FakeFileSystem


def test_absolute_imports():
    all_modules = {
        Module('foo.one'),
        Module('foo.two'),
    }
    file_system = FakeFileSystem(
        content_map={
            '/path/to/foo/one.py': """
                import foo.two
                arbitrary_expression = 1          
            """,
        },
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one'))

    assert result == {
        DirectImport(importer=Module('foo.one'), imported=Module('foo.two'))
    }


def test_absolute_from_imports():
    all_modules = {
        Module('foo.one.blue'),
        Module('foo.one.green'),
        Module('foo.two.brown'),
        Module('foo.two.yellow'),
        Module('foo.three'),
    }
    file_system = FakeFileSystem(
        contents="""
            /path/to/foo/
                __init__.py
                one/
                    __init__.py
                    blue.py
                    green.py
                two/
                    __init__.py
                    brown.py
                    yellow.py
                three.py
        """,
        content_map={
            '/path/to/foo/one/blue.py': """
                from foo.one import green
                from foo.two import yellow
                from foo import three
                arbitrary_expression = 1          
            """,
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one.blue'))

    assert result == {
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.one.green')),
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.two.yellow')),
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.three')),
    }


def test_relative_from_imports():
    all_modules = {
        Module('foo.one.blue'),
        Module('foo.one.green'),
        Module('foo.two.brown'),
        Module('foo.two.yellow'),
        Module('foo.three'),
    }
    file_system = FakeFileSystem(
        contents="""
            /path/to/foo/
                __init__.py
                one/
                    __init__.py
                    blue.py
                    green.py
                two/
                    __init__.py
                    brown.py
                    yellow.py
                three.py
        """,
        content_map={
            '/path/to/foo/one/blue.py': """
                from . import green
                from ..two import yellow
                from .. import three
                arbitrary_expression = 1          
            """,
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one.blue'))

    assert result == {
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.one.green')),
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.two.yellow')),
        DirectImport(importer=Module('foo.one.blue'), imported=Module('foo.three')),
    }