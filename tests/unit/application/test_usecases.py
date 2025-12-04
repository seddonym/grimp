from unittest.mock import sentinel
import pytest  # type: ignore

from grimp.application import usecases
from grimp.application.ports.caching import Cache
from grimp.application.ports.modulefinder import ModuleFile
from grimp.domain.valueobjects import DirectImport, Module
from tests.adaptors.filesystem import FakeFileSystem
from tests.adaptors.packagefinder import BaseFakePackageFinder
from tests.config import override_settings


class TestBuildGraph:
    @pytest.mark.parametrize("include_external_packages", (True, False))
    def test_happy_path(self, include_external_packages):
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
                "/path/to/mypackage/foo/one.py": (
                    "import mypackage.foo.two.green\nfrom .. import Something"
                ),
                "/path/to/mypackage/foo/two/green.py": "import mypackage.foo.two.blue\n"
                "from external.subpackage import foobar\n"
                "import decimal",
            },
        )

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {"mypackage": {"/path/to/mypackage"}}

        with override_settings(FILE_SYSTEM=file_system, PACKAGE_FINDER=FakePackageFinder()):
            graph = usecases.build_graph(
                "mypackage", include_external_packages=include_external_packages
            )

        expected_import_map = {
            "mypackage": set(),
            "mypackage.foo": set(),
            "mypackage.foo.one": {"mypackage.foo.two.green", "mypackage"},
            "mypackage.foo.two": set(),
            "mypackage.foo.two.green": {"mypackage.foo.two.blue"},
            "mypackage.foo.two.blue": set(),
        }
        if include_external_packages:
            expected_import_map["decimal"] = set()
            expected_import_map["external"] = set()
            expected_import_map["mypackage.foo.two.green"] |= {"external", "decimal"}

        assert set(expected_import_map.keys()) == graph.modules
        for importer, imported_modules in expected_import_map.items():
            assert graph.find_modules_directly_imported_by(importer) == imported_modules

        # Check that the external packages are squashed modules.
        if include_external_packages:
            for module in ("external", "decimal"):
                with pytest.raises(ValueError, match="Cannot find children of a squashed module."):
                    graph.find_children(module)

    def test_boolean_additional_package_raises_type_error(self):
        """
        Tests that a useful error message if build_graph is called
        with a boolean as the second argument.

        This is because earlier versions of build_graph took include_external_packages
        as the second argument, and it's possible it might have been called
        as a positional argument.
        """
        with pytest.raises(TypeError, match="Package names must be strings, got bool."):
            usecases.build_graph("mypackage", True)

    @pytest.mark.parametrize(
        "supplied_cache_dir", ("/path/to/somewhere", None, sentinel.not_supplied)
    )
    def test_build_graph_respects_cache_dir(self, supplied_cache_dir):
        file_system = FakeFileSystem()

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {"mypackage": {"/path/to/mypackage"}}

        SOME_DEFAULT_CACHE_DIR = ".some_default"

        class AssertingCache(Cache):
            @classmethod
            def cache_dir_or_default(cls, cache_dir: str | None) -> str:
                return cache_dir or SOME_DEFAULT_CACHE_DIR

            def __init__(self, *args, **kwargs) -> None:
                super().__init__(*args, **kwargs)

                # Assertions.

                if supplied_cache_dir is None:
                    raise RuntimeError("Cache should not be instantiated if caching is disabled.")

                expected_cache_dir = (
                    SOME_DEFAULT_CACHE_DIR
                    if supplied_cache_dir is sentinel.not_supplied
                    else supplied_cache_dir
                )
                assert self.cache_dir == expected_cache_dir

            def read_imports(self, module_file: ModuleFile) -> set[DirectImport]:
                return set()

            def write(
                self,
                imports_by_module: dict[Module, set[DirectImport]],
            ) -> None:
                pass

        with override_settings(
            FILE_SYSTEM=file_system,
            PACKAGE_FINDER=FakePackageFinder(),
            CACHE_CLASS=AssertingCache,
        ):
            kwargs = dict(
                include_external_packages=True,
            )
            if supplied_cache_dir is not sentinel.not_supplied:
                kwargs["cache_dir"] = supplied_cache_dir
            usecases.build_graph("mypackage", **kwargs)

    def test_forgives_wrong_type_being_passed_to_include_external_packages(self):
        file_system = FakeFileSystem(
            contents="""
                /path/to/mypackage/
                    __init__.py
                    foo/
                        __init__.py
                        one.py
                """,
            content_map={
                "/path/to/mypackage/foo/one.py": (
                    "import mypackage.foo.two.green\nfrom .. import Something"
                ),
            },
        )

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {"mypackage": {"/path/to/mypackage"}}

        with override_settings(FILE_SYSTEM=file_system, PACKAGE_FINDER=FakePackageFinder()):
            graph = usecases.build_graph(
                "mypackage",
                # Note: this should be a bool, but we want to tolerate it,
                # as Import Linter currently has a bug where it will pass it as None.
                include_external_packages=False,
            )

        expected_modules = {
            "mypackage",
            "mypackage.foo",
            "mypackage.foo.one",
        }
        assert expected_modules == graph.modules

    def test_namespace_package_passed_as_root(self):
        file_system = FakeFileSystem(
            contents="""
                /path/to/mypackage/
                    foo/
                        __init__.py
                        one.py
                        two/
                            __init__.py
                            green.py
                            blue.py
                    bar/
                        __init__.py
                        three.py
                /different-path/to/mypackage/
                    foobar/
                        __init__.py
                        four.py
                """,
        )

        class FakePackageFinder(BaseFakePackageFinder):
            directory_map = {
                "mypackage": {
                    "/path/to/mypackage",
                    "/different-path/to/mypackage",
                }
            }

        with override_settings(FILE_SYSTEM=file_system, PACKAGE_FINDER=FakePackageFinder()):
            graph = usecases.build_graph("mypackage")

        assert graph.modules == {
            # TODO "mypackage" should also be included.
            "mypackage.foo",
            "mypackage.foo.one",
            "mypackage.foo.two",
            "mypackage.foo.two.green",
            "mypackage.foo.two.blue",
            "mypackage.bar",
            "mypackage.bar.three",
            "mypackage.foobar",
            "mypackage.foobar.four",
        }
