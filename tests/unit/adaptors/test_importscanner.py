import pytest  # type: ignore

from grimp.adaptors.importscanner import ImportScanner
from grimp.domain.valueobjects import DirectImport, Module

from tests.adaptors.filesystem import FakeFileSystem


@pytest.mark.parametrize(
    'include_external_packages, expected_result',
    (
        (
            False,
            {
                DirectImport(
                    importer=Module('foo.one'),
                    imported=Module('foo.two'),
                    line_number=1,
                    line_contents='import foo.two',
                ),
            }
        ),
        (
            True,
            {
                DirectImport(
                    importer=Module('foo.one'),
                    imported=Module('foo.two'),
                    line_number=1,
                    line_contents='import foo.two',
                ),
                DirectImport(
                    importer=Module('foo.one'),
                    imported=Module('externalone'),
                    line_number=2,
                    line_contents='import externalone',
                ),
                DirectImport(
                    importer=Module('foo.one'),
                    imported=Module('externaltwo'),
                    line_number=3,
                    line_contents='import externaltwo.subpackage',
                ),
            }
        ),
    )
)
def test_absolute_imports(include_external_packages, expected_result):
    all_modules = {
        Module('foo.one'),
        Module('foo.two'),
    }
    file_system = FakeFileSystem(
        content_map={
            '/path/to/foo/one.py': """
                import foo.two
                import externalone
                import externaltwo.subpackage
                arbitrary_expression = 1
            """,
        },
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    result = import_scanner.scan_for_imports(Module('foo.one'))

    assert expected_result == result


@pytest.mark.parametrize(
    'include_external_packages, expected_result',
    (
        (
            False,
            {
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.one.green'),
                    line_number=1,
                    line_contents='from foo.one import green',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.two.yellow'),
                    line_number=2,
                    line_contents='from foo.two import yellow',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.three'),
                    line_number=3,
                    line_contents='from foo import three',
                ),
            },
        ),
        (
            True,
            {
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.one.green'),
                    line_number=1,
                    line_contents='from foo.one import green',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.two.yellow'),
                    line_number=2,
                    line_contents='from foo.two import yellow',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('foo.three'),
                    line_number=3,
                    line_contents='from foo import three',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('external'),
                    line_number=4,
                    line_contents='from external import one',
                ),
                DirectImport(
                    importer=Module('foo.one.blue'),
                    imported=Module('external'),
                    line_number=5,
                    line_contents='from external.two import blue',
                ),
            },
        )
    )
)
def test_absolute_from_imports(include_external_packages, expected_result):
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
                from external import one
                from external.two import blue
                arbitrary_expression = 1
            """,
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    result = import_scanner.scan_for_imports(Module('foo.one.blue'))

    assert expected_result == result


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
        DirectImport(
            importer=Module('foo.one.blue'),
            imported=Module('foo.one.green'),
            line_number=1,
            line_contents='from . import green',
        ),
        DirectImport(
            importer=Module('foo.one.blue'),
            imported=Module('foo.two.yellow'),
            line_number=2,
            line_contents='from ..two import yellow',
        ),
        DirectImport(
            importer=Module('foo.one.blue'),
            imported=Module('foo.three'),
            line_number=3,
            line_contents='from .. import three',
        ),
    }


@pytest.mark.parametrize(
    'import_source', (
        'from .two.yellow import my_function',
        'from foo.two.yellow import my_function',
    )
)
def test_trims_to_known_modules(import_source):
    all_modules = {
        Module('foo'),
        Module('foo.one'),
        Module('foo.two'),
        Module('foo.two.yellow'),
    }
    file_system = FakeFileSystem(
        contents="""
                /path/to/foo/
                    __init__.py
                    one.py
                    two/
                        __init__.py
                        yellow.py
            """,
        content_map={
            '/path/to/foo/one.py': import_source,
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one'))

    assert result == {
        DirectImport(
            importer=Module('foo.one'),
            imported=Module('foo.two.yellow'),
            line_number=1,
            line_contents=import_source,
        ),
    }


def test_trims_to_known_modules_within_init_file():
    all_modules = {
        Module('foo'),
        Module('foo.one'),
        Module('foo.one.yellow'),
        Module('foo.one.blue'),
        Module('foo.one.blue.alpha'),
    }
    file_system = FakeFileSystem(
        contents="""
                /path/to/foo/
                    __init__.py
                    one/
                        __init__.py
                        yellow.py
                        blue/
                            __init__.py
                            alpha.py
            """,
        content_map={
            '/path/to/foo/one/__init__.py': 'from .yellow import my_function',
            '/path/to/foo/one/blue/__init__.py': 'from .alpha import my_function',
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one'))

    assert result == {
        DirectImport(
            importer=Module('foo.one'),
            imported=Module('foo.one.yellow'),
            line_number=1,
            line_contents='from .yellow import my_function',
        ),
    }

    result = import_scanner.scan_for_imports(Module('foo.one.blue'))

    assert result == {
        DirectImport(
            importer=Module('foo.one.blue'),
            imported=Module('foo.one.blue.alpha'),
            line_number=1,
            line_contents='from .alpha import my_function',
        ),
    }


def test_trims_whitespace_from_start_of_line_contents():
    all_modules = {
        Module('foo'),
        Module('foo.one'),
        Module('foo.two'),
    }
    file_system = FakeFileSystem(
        contents="""
                    /path/to/foo/
                        __init__.py
                        one.py
                        two.py
                """,
        content_map={
            '/path/to/foo/one.py': """
            def my_function():
                from . import two
            """,
        }
    )

    import_scanner = ImportScanner(
        modules=all_modules,
        package_directory='/path/to/foo',
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module('foo.one'))

    assert result == {
        DirectImport(
            importer=Module('foo.one'),
            imported=Module('foo.two'),
            line_number=2,
            line_contents='from . import two',
        ),
    }
