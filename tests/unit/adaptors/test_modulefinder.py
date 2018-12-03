from grimp.adaptors.modulefinder import ModuleFinder
from grimp.domain.valueobjects import Module


from tests.adaptors.filesystem import FakeFileSystem


def test_happy_path():
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(contents="""
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
        """)

    result = module_finder.find_modules(
        package_name='mypackage',
        package_directory='/path/to/mypackage',
        file_system=file_system,
    )

    expected_modules = {
        Module('mypackage'),
        Module('mypackage.foo'),
        Module('mypackage.foo.one'),
        Module('mypackage.foo.two'),
        Module('mypackage.foo.two.green'),
        Module('mypackage.foo.two.blue'),
    }
    assert set(result) == expected_modules


def test_ignores_orphaned_python_files():
    # Python files in directories that don't contain an __init__.py should not be discovered.
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(contents="""
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
            """)

    result = module_finder.find_modules(
        package_name='mypackage',
        package_directory='/path/to/mypackage',
        file_system=file_system,
    )

    expected_modules = {
        Module('mypackage'),
        Module('mypackage.two'),
        Module('mypackage.two.green'),
    }
    assert set(result) == expected_modules


def test_ignores_hidden_directories():
    module_finder = ModuleFinder()

    file_system = FakeFileSystem(contents="""
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
                """)

    result = module_finder.find_modules(
        package_name='mypackage',
        package_directory='/path/to/mypackage',
        file_system=file_system,
    )

    expected_modules = {
        Module('mypackage'),
        Module('mypackage.two'),
        Module('mypackage.two.green'),
    }
    assert set(result) == expected_modules
