from typing import Set


import pytest  # type: ignore

from grimp.adaptors.importscanner import ImportScanner
from grimp.application.ports.modulefinder import FoundPackage, ModuleFile
from grimp.domain.valueobjects import DirectImport, Module

from grimp import _rustgrimp as rust  # type: ignore[attr-defined]


@pytest.mark.parametrize(
    "include_external_packages, expected_result",
    (
        (
            False,
            {
                DirectImport(
                    importer=Module("foo.one"),
                    imported=Module("foo.two"),
                    line_number=1,
                    line_contents="import foo.two",
                )
            },
        ),
        (
            True,
            {
                DirectImport(
                    importer=Module("foo.one"),
                    imported=Module("foo.two"),
                    line_number=1,
                    line_contents="import foo.two",
                ),
                DirectImport(
                    importer=Module("foo.one"),
                    imported=Module("externalone"),
                    line_number=2,
                    line_contents="import externalone",
                ),
                DirectImport(
                    importer=Module("foo.one"),
                    imported=Module("externaltwo"),
                    line_number=3,
                    line_contents="import externaltwo.subpackage  # with comment afterwards.",
                ),
            },
        ),
    ),
)
def test_absolute_imports(include_external_packages, expected_result):
    all_modules = {Module("foo.one"), Module("foo.two")}
    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/foo/one.py": """
                import foo.two
                import externalone
                import externaltwo.subpackage  # with comment afterwards.
                arbitrary_expression = 1
            """
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    result = import_scanner.scan_for_imports(Module("foo.one"))

    assert expected_result == result


def test_non_ascii():
    blue_module = Module("mypackage.blue")
    non_ascii_modules = {Module("mypackage.ñon_ascii_变"), Module("mypackage.ñon_ascii_变.ラーメン")}
    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/mypackage/blue.py": """
                from ñon_ascii_变 import *
                from . import ñon_ascii_变
                import mypackage.ñon_ascii_变.ラーメン
            """,
            "/path/to/mypackage/ñon_ascii_变/__init__.py": "",
            "/path/to/mypackage/ñon_ascii_变/ラーメン.py": "",
        },
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="mypackage",
                directory="/path/to/mypackage",
                module_files=frozenset(
                    _modules_to_module_files({blue_module} | non_ascii_modules)
                ),
            )
        },
        file_system=file_system,
        include_external_packages=True,
    )

    result = import_scanner.scan_for_imports(blue_module)

    assert result == {
        DirectImport(
            importer=Module("mypackage.blue"),
            imported=Module("ñon_ascii_变"),
            line_number=1,
            line_contents="from ñon_ascii_变 import *",
        ),
        DirectImport(
            importer=Module("mypackage.blue"),
            imported=Module("mypackage.ñon_ascii_变"),
            line_number=2,
            line_contents="from . import ñon_ascii_变",
        ),
        DirectImport(
            importer=Module("mypackage.blue"),
            imported=Module("mypackage.ñon_ascii_变.ラーメン"),
            line_number=3,
            line_contents="import mypackage.ñon_ascii_变.ラーメン",
        ),
    }


def test_single_namespace_package_portion():
    MODULE_FOO = Module("namespace.foo")
    MODULE_ONE = Module("namespace.foo.one")
    MODULE_TWO = Module("namespace.foo.two")
    MODULE_THREE = Module("namespace.foo.three")
    MODULE_FOUR = Module("namespace.foo.four")
    MODULE_GREEN = Module("namespace.foo.two.green")
    MODULE_BLUE = Module("namespace.foo.two.blue")
    MODULE_ALPHA = Module("namespace.foo.two.green.alpha")

    all_modules = {
        MODULE_FOO,
        MODULE_ONE,
        MODULE_TWO,
        MODULE_THREE,
        MODULE_FOUR,
        MODULE_GREEN,
        MODULE_BLUE,
        MODULE_ALPHA,
    }

    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/namespace/foo/one.py": """
                import namespace.foo.two
                from namespace.foo import three
                from . import four
            """,
            "/path/to/namespace/foo/two/green/alpha.py": """
                from .. import blue
                from ... import three
            """,
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="namespace.foo",
                directory="/path/to/namespace/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    results = (
        import_scanner.scan_for_imports(MODULE_ONE),
        import_scanner.scan_for_imports(MODULE_ALPHA),
    )

    assert results == (
        {
            DirectImport(
                importer=MODULE_ONE,
                imported=MODULE_TWO,
                line_number=1,
                line_contents="import namespace.foo.two",
            ),
            DirectImport(
                importer=MODULE_ONE,
                imported=MODULE_THREE,
                line_number=2,
                line_contents="from namespace.foo import three",
            ),
            DirectImport(
                importer=MODULE_ONE,
                imported=MODULE_FOUR,
                line_number=3,
                line_contents="from . import four",
            ),
        },
        {
            DirectImport(
                importer=MODULE_ALPHA,
                imported=MODULE_BLUE,
                line_number=1,
                line_contents="from .. import blue",
            ),
            DirectImport(
                importer=MODULE_ALPHA,
                imported=MODULE_THREE,
                line_number=2,
                line_contents="from ... import three",
            ),
        },
    )


@pytest.mark.parametrize("include_external_packages", (True, False))
def test_import_of_portion_not_in_graph(include_external_packages):
    # Include two namespaces, one deeper than the other.
    MODULE_BAR_ONE_GREEN = Module("namespace.bar.one.green")
    MODULE_BAR_ONE_GREEN_ALPHA = Module("namespace.bar.one.green.alpha")

    MODULE_FOO = Module("namespace.foo")
    MODULE_FOO_ONE = Module("namespace.foo.one")
    MODULE_FOO_BLUE = Module("namespace.foo.one.blue")

    all_modules = {
        MODULE_FOO,
        MODULE_FOO_ONE,
        MODULE_FOO_BLUE,
    }
    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/namespace/foo/one.py": """
                import namespace.bar.one.orange
                from namespace.yellow import one
                from .. import mauve
                if t.TYPE_CHECKING:
                    from ..cyan import one
                from ..magenta.one import alpha
            """,
            "/path/to/namespace/foo/one/blue.py": """
                from ... import teal
                from ...pink import one
                from ...scarlet.one import alpha
            """,
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="namespace.foo",
                directory="/path/to/namespace/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            ),
            FoundPackage(
                name="namespace.bar.one.green",
                directory="/path/to/namespace/bar/one/green",
                module_files=frozenset(
                    _modules_to_module_files(
                        {
                            MODULE_BAR_ONE_GREEN,
                            MODULE_BAR_ONE_GREEN_ALPHA,
                        }
                    )
                ),
            ),
        },
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    results = (
        import_scanner.scan_for_imports(MODULE_FOO_ONE),
        import_scanner.scan_for_imports(MODULE_FOO_BLUE),
    )

    if include_external_packages:
        assert results == (
            {
                DirectImport(
                    importer=MODULE_FOO_ONE,
                    imported=Module("namespace.bar.one.orange"),
                    line_number=1,
                    line_contents="import namespace.bar.one.orange",
                ),
                DirectImport(
                    importer=MODULE_FOO_ONE,
                    imported=Module("namespace.yellow"),
                    line_number=2,
                    line_contents="from namespace.yellow import one",
                ),
                DirectImport(
                    importer=MODULE_FOO_ONE,
                    imported=Module("namespace.mauve"),
                    line_number=3,
                    line_contents="from .. import mauve",
                ),
                DirectImport(
                    importer=MODULE_FOO_ONE,
                    imported=Module("namespace.cyan"),
                    line_number=5,
                    line_contents="from ..cyan import one",
                ),
                DirectImport(
                    importer=MODULE_FOO_ONE,
                    imported=Module("namespace.magenta"),
                    line_number=6,
                    line_contents="from ..magenta.one import alpha",
                ),
            },
            {
                DirectImport(
                    importer=MODULE_FOO_BLUE,
                    imported=Module("namespace.teal"),
                    line_number=1,
                    line_contents="from ... import teal",
                ),
                DirectImport(
                    importer=MODULE_FOO_BLUE,
                    imported=Module("namespace.pink"),
                    line_number=2,
                    line_contents="from ...pink import one",
                ),
                DirectImport(
                    importer=MODULE_FOO_BLUE,
                    imported=Module("namespace.scarlet"),
                    line_number=3,
                    line_contents="from ...scarlet.one import alpha",
                ),
            },
        )
    else:
        assert results == (set(), set())


@pytest.mark.parametrize(
    "include_external_packages, expected_result",
    (
        (
            False,
            {
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.one.green"),
                    line_number=1,
                    line_contents="from foo.one import green",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.two.yellow"),
                    line_number=2,
                    line_contents="from foo.two import yellow",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.three"),
                    line_number=4,
                    line_contents="from foo import three",
                ),
            },
        ),
        (
            True,
            {
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.one.green"),
                    line_number=1,
                    line_contents="from foo.one import green",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.two.yellow"),
                    line_number=2,
                    line_contents="from foo.two import yellow",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("foo.three"),
                    line_number=4,
                    line_contents="from foo import three",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("external"),
                    line_number=5,
                    line_contents="from external import one",
                ),
                DirectImport(
                    importer=Module("foo.one.blue"),
                    imported=Module("external"),
                    line_number=6,
                    line_contents="from external.two import blue  # with comment afterwards.",
                ),
            },
        ),
    ),
)
def test_absolute_from_imports(include_external_packages, expected_result):
    all_modules = {
        Module("foo"),
        Module("foo.one"),
        Module("foo.one.blue"),
        Module("foo.one.green"),
        Module("foo.two"),
        Module("foo.two.brown"),
        Module("foo.two.yellow"),
        Module("foo.three"),
    }
    file_system = rust.FakeBasicFileSystem(
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
            "/path/to/foo/one/blue.py": """
                from foo.one import green
                from foo.two import yellow
                if t.TYPE_CHECKING:
                    from foo import three
                from external import one
                from external.two import blue  # with comment afterwards.
                arbitrary_expression = 1
            """
        },
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
        include_external_packages=include_external_packages,
    )

    result = import_scanner.scan_for_imports(Module("foo.one.blue"))

    assert expected_result == result


def test_relative_from_imports():
    all_modules = {
        Module("foo.one.blue"),
        Module("foo.one.green"),
        Module("foo.two.brown"),
        Module("foo.two.yellow"),
        Module("foo.three"),
    }
    file_system = rust.FakeBasicFileSystem(
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
            "/path/to/foo/one/blue.py": """
                from . import green
                from ..two import yellow
                from .. import three
                arbitrary_expression = 1
            """
        },
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module("foo.one.blue"))

    assert result == {
        DirectImport(
            importer=Module("foo.one.blue"),
            imported=Module("foo.one.green"),
            line_number=1,
            line_contents="from . import green",
        ),
        DirectImport(
            importer=Module("foo.one.blue"),
            imported=Module("foo.two.yellow"),
            line_number=2,
            line_contents="from ..two import yellow",
        ),
        DirectImport(
            importer=Module("foo.one.blue"),
            imported=Module("foo.three"),
            line_number=3,
            line_contents="from .. import three",
        ),
    }


@pytest.mark.parametrize(
    "import_source",
    ("from .two.yellow import my_function", "from foo.two.yellow import my_function"),
)
def test_trims_to_known_modules(import_source):
    all_modules = {
        Module("foo"),
        Module("foo.one"),
        Module("foo.two"),
        Module("foo.two.yellow"),
    }
    file_system = rust.FakeBasicFileSystem(
        contents="""
                /path/to/foo/
                    __init__.py
                    one.py
                    two/
                        __init__.py
                        yellow.py
            """,
        content_map={"/path/to/foo/one.py": import_source},
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module("foo.one"))

    assert result == {
        DirectImport(
            importer=Module("foo.one"),
            imported=Module("foo.two.yellow"),
            line_number=1,
            line_contents=import_source,
        )
    }


def test_trims_to_known_modules_within_init_file():
    all_modules = {
        Module("foo"),
        Module("foo.one"),
        Module("foo.one.yellow"),
        Module("foo.one.blue"),
        Module("foo.one.blue.alpha"),
    }
    file_system = rust.FakeBasicFileSystem(
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
            "/path/to/foo/one/__init__.py": "from .yellow import my_function",
            "/path/to/foo/one/blue/__init__.py": "from .alpha import my_function",
        },
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module("foo.one"))

    assert result == {
        DirectImport(
            importer=Module("foo.one"),
            imported=Module("foo.one.yellow"),
            line_number=1,
            line_contents="from .yellow import my_function",
        )
    }

    result = import_scanner.scan_for_imports(Module("foo.one.blue"))

    assert result == {
        DirectImport(
            importer=Module("foo.one.blue"),
            imported=Module("foo.one.blue.alpha"),
            line_number=1,
            line_contents="from .alpha import my_function",
        )
    }


def test_trims_whitespace_from_start_of_line_contents():
    all_modules = {Module("foo"), Module("foo.one"), Module("foo.two")}
    file_system = rust.FakeBasicFileSystem(
        contents="""
                    /path/to/foo/
                        __init__.py
                        one.py
                        two.py
                """,
        content_map={
            "/path/to/foo/one.py": """
            def my_function():
                from . import two
            """
        },
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module("foo.one"))

    assert result == {
        DirectImport(
            importer=Module("foo.one"),
            imported=Module("foo.two"),
            line_number=2,
            line_contents="from . import two",
        )
    }


@pytest.mark.parametrize(
    "statement, expected_module_name",
    (
        # External packages that share a namespace with an internal module resolve
        # to the shallowest component that does not clash with an internal module namespace.
        ("import namespace", None),
        ("import namespace.foo", None),
        ("from namespace import foo", None),
        ("import namespace.bar", "namespace.bar"),
        ("from namespace import bar", "namespace.bar"),
        ("from ... import bar", "namespace.bar"),
        ("import namespace.bar.orange", "namespace.bar"),
        ("from namespace.bar import orange", "namespace.bar"),
        ("from ...bar import orange", "namespace.bar"),
        ("import namespace.foo.green", "namespace.foo.green"),
        ("from namespace.foo import green", "namespace.foo.green"),
        ("from .. import green", "namespace.foo.green"),
        ("import namespace.foo.green.alpha", "namespace.foo.green"),
        ("from namespace.foo.green import alpha", "namespace.foo.green"),
        ("from ..green import alpha", "namespace.foo.green"),
        ("import namespace.foo.green.alpha.one", "namespace.foo.green"),
        ("from namespace.foo.green.alpha import one", "namespace.foo.green"),
        ("from ..green.alpha import one", "namespace.foo.green"),
        ("from .. import green", "namespace.foo.green"),
        ("import namespace.foo.green.alpha", "namespace.foo.green"),
        ("from namespace.foo.green import alpha", "namespace.foo.green"),
        ("from ..green import alpha", "namespace.foo.green"),
        ("import namespace.foo.green.alpha.one", "namespace.foo.green"),
        ("from namespace.foo.green.alpha import one", "namespace.foo.green"),
        ("from ..green.alpha import one", "namespace.foo.green"),
    ),
)
def test_external_package_imports_for_namespace_packages(statement, expected_module_name):
    module_to_scan = Module("namespace.foo.blue.alpha")

    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/namespace/foo/blue/alpha.py": statement,
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="namespace.foo.blue",
                directory="/path/to/namespace/foo/blue",
                module_files=frozenset(
                    _modules_to_module_files(
                        {
                            Module("namespace.foo.blue"),
                            module_to_scan,
                            Module("namespace.foo.blue.beta"),
                        }
                    )
                ),
            )
        },
        file_system=file_system,
        include_external_packages=True,
    )

    result = import_scanner.scan_for_imports(module_to_scan)

    if expected_module_name:
        assert {
            DirectImport(
                importer=module_to_scan,
                imported=Module(expected_module_name),
                line_number=1,
                line_contents=statement,
            ),
        } == result
    else:
        assert result == set()


@pytest.mark.parametrize("statement", ("import bar.blue", "from bar import blue"))
def test_scans_multiple_packages(statement):
    foo_modules = {Module("foo"), Module("foo.one"), Module("foo.two")}
    bar_modules = {Module("bar"), Module("bar.green"), Module("bar.blue")}
    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/foo/one.py": f"""
                import foo.two
                {statement}
                import externalone

                arbitrary_expression = 1
            """
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(foo_modules)),
            ),
            FoundPackage(
                name="bar",
                directory="/path/to/bar",
                module_files=frozenset(_modules_to_module_files(bar_modules)),
            ),
        },
        file_system=file_system,
    )

    result = import_scanner.scan_for_imports(Module("foo.one"))

    assert {
        DirectImport(
            importer=Module("foo.one"),
            imported=Module("foo.two"),
            line_number=1,
            line_contents="import foo.two",
        ),
        DirectImport(
            importer=Module("foo.one"),
            imported=Module("bar.blue"),
            line_number=2,
            line_contents=statement,
        ),
    } == result


@pytest.mark.parametrize("exclude_type_checking_imports", (True, False))
@pytest.mark.parametrize(
    "statement, is_statement_valid",
    (
        ("if t.TYPE_CHECKING:", True),
        ("if TYPE_CHECKING:", True),
        ("if typing.TYPE_CHECKING:", True),
        ("if WEIRD_ALIAS.TYPE_CHECKING:", True),
        ("if type_checking:", False),
        ("while TYPE_CHECKING:", False),
    ),
)
def test_exclude_type_checking_imports(
    exclude_type_checking_imports, statement, is_statement_valid
):
    all_modules = {
        Module("foo.one"),
        Module("foo.two"),
        Module("foo.three"),
        Module("foo.four"),
        Module("foo.five"),
    }
    file_system = rust.FakeBasicFileSystem(
        content_map={
            "/path/to/foo/one.py": f"""
                import foo.two
                {statement}
                    import foo.three
                    import foo.four
                import foo.five
                arbitrary_expression = 1
            """
        }
    )

    import_scanner = ImportScanner(
        found_packages={
            FoundPackage(
                name="foo",
                directory="/path/to/foo",
                module_files=frozenset(_modules_to_module_files(all_modules)),
            )
        },
        file_system=file_system,
    )

    if exclude_type_checking_imports and is_statement_valid:
        expected_result = {
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.two"),
                line_number=1,
                line_contents="import foo.two",
            ),
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.five"),
                line_number=5,
                line_contents="import foo.five",
            ),
        }
    else:
        expected_result = {
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.two"),
                line_number=1,
                line_contents="import foo.two",
            ),
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.three"),
                line_number=3,
                line_contents="import foo.three",
            ),
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.four"),
                line_number=4,
                line_contents="import foo.four",
            ),
            DirectImport(
                importer=Module("foo.one"),
                imported=Module("foo.five"),
                line_number=5,
                line_contents="import foo.five",
            ),
        }

    result = import_scanner.scan_for_imports(
        Module("foo.one"), exclude_type_checking_imports=exclude_type_checking_imports
    )

    assert expected_result == result


def _modules_to_module_files(modules: Set[Module]) -> Set[ModuleFile]:
    some_mtime = 100933.4
    return {ModuleFile(module=module, mtime=some_mtime) for module in modules}
