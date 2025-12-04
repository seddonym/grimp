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


def test_namespaced_packages():
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(
        contents="""
        /path/to/somenamespace/foo/
                __init__.py
                blue.py
                green/
                    __init__.py
                    one.py
                    two/
                        __init__.py
        """
    )

    result = module_finder.find_package(
        package_name="somenamespace.foo",
        package_directory="/path/to/somenamespace/foo",
        file_system=file_system,
    )

    assert result == FoundPackage(
        name="somenamespace.foo",
        directory="/path/to/somenamespace/foo",
        module_files={
            ModuleFile(module=Module("somenamespace.foo"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("somenamespace.foo.blue"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("somenamespace.foo.green"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("somenamespace.foo.green.one"), mtime=DEFAULT_MTIME),
            ModuleFile(module=Module("somenamespace.foo.green.two"), mtime=DEFAULT_MTIME),
        },
    )


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
