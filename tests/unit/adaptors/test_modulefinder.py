from grimp.adaptors.modulefinder import ModuleFinder
from grimp.application.ports.modulefinder import FoundPackage, ModuleFile
from grimp.domain.valueobjects import Module
from tests.adaptors.filesystem import DEFAULT_MTIME, FakeFileSystem
import pytest


def test_happy_path():
    module_finder = ModuleFinder()
    SOME_MTIME = 12340000.3
    file_system = FakeFileSystem(
        contents="""
        /path/to/mypackage/
            __init__.py
            not-a-python-file.txt
            .hidden
            foo/
                __init__.py
                one.py
                two/
                    __init__.py
                    green.py
                    blue.py
        """,
        mtime_map={
            "/path/to/mypackage/foo/one.py": SOME_MTIME,
        },
    )

    result = module_finder.find_package(
        package_name="mypackage",
        package_directory="/path/to/mypackage",
        file_system=file_system,
    )

    assert result == FoundPackage(
        name="mypackage",
        directory="/path/to/mypackage",
        module_files=frozenset(
            {
                ModuleFile(module=Module("mypackage"), mtime=DEFAULT_MTIME),
                ModuleFile(module=Module("mypackage.foo"), mtime=DEFAULT_MTIME),
                ModuleFile(module=Module("mypackage.foo.one"), mtime=SOME_MTIME),
                ModuleFile(module=Module("mypackage.foo.two"), mtime=DEFAULT_MTIME),
                ModuleFile(module=Module("mypackage.foo.two.green"), mtime=DEFAULT_MTIME),
                ModuleFile(module=Module("mypackage.foo.two.blue"), mtime=DEFAULT_MTIME),
            }
        ),
    )


MODULE_FILES_FOO_BLUE = {
    ModuleFile(module=Module("somenamespace.foo.blue"), mtime=DEFAULT_MTIME),
    ModuleFile(module=Module("somenamespace.foo.blue.one"), mtime=DEFAULT_MTIME),
    ModuleFile(module=Module("somenamespace.foo.blue.two"), mtime=DEFAULT_MTIME),
    ModuleFile(module=Module("somenamespace.foo.blue.two.alpha"), mtime=DEFAULT_MTIME),
}
MODULE_FILES_FOO_GREEN_FIVE = {
    ModuleFile(module=Module("somenamespace.foo.green.five"), mtime=DEFAULT_MTIME),
    ModuleFile(module=Module("somenamespace.foo.green.five.beta"), mtime=DEFAULT_MTIME),
}


@pytest.mark.parametrize(
    "package_name, package_directory, expected",
    [
        (
            "somenamespace",
            "/path/to/somenamespace",
            FoundPackage(
                name="somenamespace",
                directory="/path/to/somenamespace",
                module_files=MODULE_FILES_FOO_BLUE | MODULE_FILES_FOO_GREEN_FIVE,
                namespace_packages=frozenset(
                    {
                        "somenamespace",
                        "somenamespace.foo",
                        "somenamespace.foo.green",
                    }
                ),
            ),
        ),
        (
            "somenamespace.foo",
            "/path/to/somenamespace/foo",
            FoundPackage(
                name="somenamespace.foo",
                directory="/path/to/somenamespace/foo",
                module_files=MODULE_FILES_FOO_BLUE | MODULE_FILES_FOO_GREEN_FIVE,
                namespace_packages=frozenset(
                    {
                        "somenamespace.foo",
                        "somenamespace.foo.green",
                    }
                ),
            ),
        ),
        (
            "somenamespace.foo.blue",
            "/path/to/somenamespace/foo/blue",
            FoundPackage(
                name="somenamespace.foo.blue",
                directory="/path/to/somenamespace/foo/blue",
                module_files=MODULE_FILES_FOO_BLUE,
            ),
        ),
    ],
)
def test_namespaced_packages(package_name: str, package_directory: str, expected: FoundPackage):
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(
        contents="""
            /path/to/somenamespace/
                foo/
                    blue/
                        __init__.py
                        one.py
                        two/
                            __init__.py
                            alpha.py
                        noinitpackage/
                            three.py
                            orphan/
                                __init__.py
                                four.py
                    green/
                        five/
                            __init__.py
                            beta.py
        """
    )

    result = module_finder.find_package(
        package_name=package_name,
        package_directory=package_directory,
        file_system=file_system,
    )

    assert result == expected


def test_ignores_orphaned_python_files():
    # Python files in directories that don't contain an __init__.py should not be discovered.
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(
        contents="""
            /path/to/mypackage/
                __init__.py
                two/
                    __init__.py
                    green.py
                noinitpackage/
                    green.py
                    orphan/
                        __init__.py
                        red.py
            """
    )

    result = module_finder.find_package(
        package_name="mypackage",
        package_directory="/path/to/mypackage",
        file_system=file_system,
    )

    assert result == FoundPackage(
        name="mypackage",
        directory="/path/to/mypackage",
        module_files={
            ModuleFile(module=Module("mypackage"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("mypackage.two"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("mypackage.two.green"), mtime=DEFAULT_MTIME),
        },
    )


@pytest.mark.parametrize(
    "extension, should_warn",
    (
        ("py", True),
        ("txt", False),  # Anything other than .py.
    ),
)
def test_ignores_dotted_python_files(extension, should_warn, caplog):
    # Python files containing dots (other than the one before .py) should not be discovered.
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(
        contents=f"""
            /path/to/mypackage/
                __init__.py
                foo.py
                bar/
                    __init__.py
                    baz.dotted.{extension}
            """
    )

    result = module_finder.find_package(
        package_name="mypackage",
        package_directory="/path/to/mypackage",
        file_system=file_system,
    )

    assert result == FoundPackage(
        name="mypackage",
        directory="/path/to/mypackage",
        module_files={
            ModuleFile(module=Module("mypackage"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("mypackage.foo"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("mypackage.bar"), mtime=DEFAULT_MTIME),
        },
    )
    if should_warn:
        assert caplog.messages == [
            (
                "Warning: skipping module with too many dots in the name: "
                f"/path/to/mypackage/bar/baz.dotted.{extension}"
            )
        ]
    else:
        assert caplog.messages == []


def test_ignores_hidden_directories():
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(
        contents="""
                /path/to/mypackage/
                    __init__.py
                    two/
                        __init__.py
                        green.py
                    .hidden/
                        green.py
                        orphan/
                            __init__.py
                            red.py
                """
    )

    result = module_finder.find_package(
        package_name="mypackage",
        package_directory="/path/to/mypackage",
        file_system=file_system,
    )

    assert result == FoundPackage(
        name="mypackage",
        directory="/path/to/mypackage",
        module_files={
            ModuleFile(Module("mypackage"), mtime=DEFAULT_MTIME),
            ModuleFile(Module("mypackage.two"), mtime=DEFAULT_MTIME),
            ModuleFile(Module("mypackage.two.green"), mtime=DEFAULT_MTIME),
        },
    )
