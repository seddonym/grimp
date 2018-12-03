from grimp.application import usecases

from tests.adaptors.filesystem import FakeFileSystem
from tests.adaptors.packagefinder import BaseFakePackageFinder
from tests.config import override_settings


class TestBuildGraph:
    def test_happy_path(self):
        file_system = FakeFileSystem(
            contents="""
                /path/to/mypackage/
                    __init__.py
                    foo/
                        __init__.py
                        one.py
                        two/
                            __init__.py
                            green.py
                            blue.py
            """,
            content_map={
                '/path/to/mypackage/foo/one.py': 'import mypackage.foo.two.green',
                '/path/to/mypackage/foo/two/green.py': 'import mypackage.foo.two.blue',
            }
        )

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {
                'mypackage': '/path/to/mypackage',
            }

        with override_settings(
            FILE_SYSTEM=file_system,
            PACKAGE_FINDER=FakePackageFinder(),
        ):
            graph = usecases.build_graph('mypackage')

        expected_import_map = {
            'mypackage': set(),
            'mypackage.foo': set(),
            'mypackage.foo.one': {'mypackage.foo.two.green'},
            'mypackage.foo.two': set(),
            'mypackage.foo.two.green': {'mypackage.foo.two.blue'},
            'mypackage.foo.two.blue': set(),
        }

        assert set(expected_import_map.keys()) == graph.modules
        for importer, imported_modules in expected_import_map.items():
            assert graph.find_modules_directly_imported_by(importer) == imported_modules
