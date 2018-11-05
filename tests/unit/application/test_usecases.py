from grimp.application import usecases
from grimp.domain.valueobjects import Module

from tests.adaptors.filesystem import FakeFileSystem
from tests.adaptors.importscanner import BaseFakeImportScanner
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
            """
        )

        class FakeImportScanner(BaseFakeImportScanner):
            import_map = {
                Module('mypackage'): set(),
                Module('mypackage.foo'): set(),
                Module('mypackage.foo.one'): {Module('mypackage.foo.two.green')},
                Module('mypackage.foo.two'): set(),
                Module('mypackage.foo.two.green'): {Module('mypackage.foo.two.blue')},
                Module('mypackage.foo.two.blue'): set(),
            }

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {
                'mypackage': '/path/to/mypackage',
            }

        with override_settings(
            FILE_SYSTEM=file_system,
            IMPORT_SCANNER_CLASS=FakeImportScanner,
            PACKAGE_FINDER=FakePackageFinder(),
        ):
            graph = usecases.build_graph('mypackage')

        assert set(FakeImportScanner.import_map.keys()) == graph.modules
        for module, imported_modules in FakeImportScanner.import_map.items():
            assert graph.find_modules_directly_imported_by(module) == set(imported_modules)
