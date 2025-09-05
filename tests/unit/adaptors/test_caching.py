import json
import logging
from typing import Set

import pytest  # type: ignore

from grimp.adaptors.caching import Cache, CacheFileNamer
from grimp.application.ports.caching import CacheMiss
from grimp.application.ports.modulefinder import FoundPackage, ModuleFile
from grimp.domain.valueobjects import DirectImport, Module
from tests.adaptors.filesystem import FakeFileSystem


class SimplisticFileNamer(CacheFileNamer):
    """
    Simplistic version of the file namer that makes testing easier.

    Does not base64 encode the data filenames.
    """

    @classmethod
    def make_data_file_name(
        cls,
        found_packages: Set[FoundPackage],
        include_external_packages: bool,
        exclude_type_checking_imports: bool,
    ) -> str:
        unsafe_name = cls.make_data_file_unique_string(
            found_packages, include_external_packages, exclude_type_checking_imports
        )
        return f"{unsafe_name}.data.json"


class TestCacheFileNamer:
    def test_names_meta_cache_per_package(self):
        result = CacheFileNamer.make_meta_file_name(
            FoundPackage(
                name="some-package",
                directory="whatever",
                module_files=set(),
            )
        )

        assert result == "some-package.meta.json"

    @pytest.mark.parametrize(
        "include_external_packages, exclude_type_checking_imports, expected",
        (
            # Blake2B 20-character hash of "hyphenated-package,underscore_package".
            (False, False, "a857d066514de048b7f94fa8d385e8bd7b048406.data.json"),
            # Blake2B 20-character hash "hyphenated-package,underscore_package:external".
            (
                True,
                False,
                "021977b6de56b09810ae52f5c9d067622c1ea30f.data.json",
            ),
            # Blake2B 20-character hash
            # "hyphenated-package,underscore_package:external:no_type_checking".
            (
                True,
                True,
                "4c2deb1d787161187915e159b5a17ea8b27cd0d4.data.json",
            ),
            # Blake2B 20-character hash "hyphenated-package,underscore_package:no_type_checking".
            (
                False,
                True,
                "815e7686179e2f3f817c130eec1121d53e62ff1c.data.json",
            ),
        ),
    )
    def test_make_data_file_name(
        self, include_external_packages, exclude_type_checking_imports, expected
    ):
        result = CacheFileNamer.make_data_file_name(
            found_packages={
                FoundPackage(
                    name="hyphenated-package",
                    directory="whatever",
                    module_files=frozenset(),
                ),
                FoundPackage(
                    name="underscore_package",
                    directory="whatever",
                    module_files=frozenset(),
                ),
            },
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
        )

        assert result == expected

    @pytest.mark.parametrize(
        "include_external_packages, exclude_type_checking_imports, expected",
        (
            (False, False, "hyphenated-package,underscore_package"),
            (True, False, "hyphenated-package,underscore_package:external"),
            (True, True, "hyphenated-package,underscore_package:external:no_type_checking"),
            (False, True, "hyphenated-package,underscore_package:no_type_checking"),
        ),
    )
    def test_make_data_file_unique_string(
        self, include_external_packages, exclude_type_checking_imports, expected
    ):
        result = CacheFileNamer.make_data_file_unique_string(
            found_packages={
                FoundPackage(
                    name="hyphenated-package",
                    directory="whatever",
                    module_files=frozenset(),
                ),
                FoundPackage(
                    name="underscore_package",
                    directory="whatever",
                    module_files=frozenset(),
                ),
            },
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
        )

        assert result == expected


class TestCache:
    SOME_MTIME = 1676645081.4935088
    FILE_SYSTEM = FakeFileSystem(
        contents="""
            .grimp_cache/
                mypackage.meta.json
                mypackage.data.json
            /path/to/mypackage/
                __init__.py
                foo/
                    __init__.py
                    new.py
                    unmodified.py
                    modified.py
            /path/to/anotherpackage/
                __init__.py
                new.py
                unmodified.py
                modified.py
        """,
        content_map={
            ".grimp_cache/mypackage.meta.json": f"""{{
                "mypackage.foo.unmodified": {SOME_MTIME},
                "mypackage.foo.modified": {SOME_MTIME}
            }}""",
            ".grimp_cache/anotherpackage.meta.json": f"""{{
                "anotherpackage.unmodified": {SOME_MTIME},
                "anotherpackage.modified": {SOME_MTIME}
            }}""",
            ".grimp_cache/mypackage.data.json": """{
                "mypackage.foo.unmodified": [
                    ["yellow", 11, "import yellow"], ["brown", 22, "import brown"]
                ],
                "mypackage.foo.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ]
            }""",
            ".grimp_cache/mypackage:external.data.json": """{
                "mypackage.foo.unmodified": [
                    ["yellow", 11, "import yellow"],
                    ["brown", 22, "import brown"],
                    ["external", 100, "import external"]
                ],
                "mypackage.foo.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ]
            }""",
            ".grimp_cache/anotherpackage,mypackage.data.json": """{
                "mypackage.foo.unmodified": [
                    ["yellow", 11, "import yellow"], ["brown", 22, "import brown"]
                ],
                "mypackage.foo.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ],
                "anotherpackage.unmodified": [
                    ["purple", 11, "import purple"], ["green", 22, "import green"]
                ],
                "anotherpackage.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ]
            }""",
            ".grimp_cache/anotherpackage,mypackage:external.data.json": """{
                "mypackage.foo.unmodified": [
                    ["yellow", 11, "import yellow"],
                    ["brown", 22, "import brown"],
                    ["external", 100, "import external"]
                ],
                "mypackage.foo.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ],
                "anotherpackage.unmodified": [
                    ["purple", 11, "import purple"],
                    ["green", 22, "import green"],
                    ["anotherexternal", 100, "import anotherexternal"]
                ],
                "anotherpackage.modified": [
                    ["stale", 33, "We should not use this cached value."]
                ]
            }""",
        },
    ).convert_to_basic()
    MODULE_FILE_UNMODIFIED = ModuleFile(
        module=Module("mypackage.foo.unmodified"), mtime=SOME_MTIME
    )
    MODULE_FILE_MODIFIED = ModuleFile(
        module=Module("mypackage.foo.modified"), mtime=SOME_MTIME + 100.0
    )
    MODULE_FILE_NEW = ModuleFile(module=Module("mypackage.foo.new"), mtime=SOME_MTIME)

    FOUND_PACKAGES = {
        FoundPackage(
            name="mypackage",
            directory="/path/to/mypackage/",
            module_files=frozenset(
                {MODULE_FILE_MODIFIED, MODULE_FILE_UNMODIFIED, MODULE_FILE_NEW}
            ),
        ),
    }

    @pytest.mark.parametrize(
        "include_external_packages, expected_data_file",
        (
            (True, ".grimp_cache/mypackage:external.data.json"),
            (False, ".grimp_cache/mypackage.data.json"),
        ),
    )
    def test_logs_successful_cache_file_reading(
        self, include_external_packages: bool, expected_data_file: str, caplog
    ):
        caplog.set_level(logging.INFO, logger=Cache.__module__)

        Cache.setup(
            file_system=self.FILE_SYSTEM,
            found_packages=self.FOUND_PACKAGES,
            include_external_packages=include_external_packages,
            namer=SimplisticFileNamer,
        )

        assert caplog.messages == [
            "Used cache meta file .grimp_cache/mypackage.meta.json.",
            f"Used cache data file {expected_data_file}.",
        ]

    def test_logs_missing_cache_files(self, caplog):
        caplog.set_level(logging.INFO, logger=Cache.__module__)

        Cache.setup(
            file_system=FakeFileSystem().convert_to_basic(),  # No cache files.
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        assert caplog.messages == [
            "No cache file: .grimp_cache/mypackage.meta.json.",
            "No cache file: .grimp_cache/mypackage.data.json.",
        ]

    @pytest.mark.parametrize("serialized_mtime", ("INVALID_JSON", '["wrong", "type"]'))
    def test_logs_corrupt_cache_meta_file_reading(self, serialized_mtime: str, caplog):
        caplog.set_level(logging.WARNING, logger=Cache.__module__)

        file_system = FakeFileSystem(
            contents="""
                .grimp_cache/
                    mypackage.meta.json
                    mypackage.data.json
                /path/to/mypackage.py
            """,
            content_map={
                ".grimp_cache/mypackage.meta.json": f"""{{
                    "mypackage.foo.modified": {serialized_mtime},
                }}""",
                ".grimp_cache/mypackage.data.json": "{}",
            },
        ).convert_to_basic()
        Cache.setup(
            file_system=file_system,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        assert caplog.messages == [
            "Could not use corrupt cache file .grimp_cache/mypackage.meta.json.",
        ]

    def test_logs_corrupt_cache_data_file_reading(self, caplog):
        caplog.set_level(logging.WARNING, logger=Cache.__module__)

        file_system = FakeFileSystem(
            contents="""
                    .grimp_cache/
                        mypackage.meta.json
                        mypackage.data.json
                    /path/to/mypackage.py
                """,
            content_map={
                ".grimp_cache/mypackage.meta.json": f"""{{
                    "mypackage.foo.modified": {self.SOME_MTIME - 1}
                }}""",
                ".grimp_cache/mypackage.data.json": "INVALID JSON",
            },
        ).convert_to_basic()

        Cache.setup(
            file_system=file_system,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        assert caplog.messages == [
            "Could not use corrupt cache file .grimp_cache/mypackage.data.json.",
        ]

    @pytest.mark.parametrize("include_external_packages", (True, False))
    def test_raises_cache_miss_for_module_with_different_mtime(
        self, include_external_packages: bool
    ):
        cache = Cache.setup(
            file_system=self.FILE_SYSTEM,
            found_packages=self.FOUND_PACKAGES,
            include_external_packages=include_external_packages,
            namer=SimplisticFileNamer,
        )
        with pytest.raises(CacheMiss):
            cache.read_imports(self.MODULE_FILE_MODIFIED)

    @pytest.mark.parametrize("include_external_packages", (True, False))
    def test_raises_cache_miss_for_module_with_no_cached_mtime(
        self, include_external_packages: bool
    ):
        cache = Cache.setup(
            file_system=self.FILE_SYSTEM,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=include_external_packages,
        )

        with pytest.raises(CacheMiss):
            cache.read_imports(self.MODULE_FILE_NEW)

    @pytest.mark.parametrize(
        "include_external_packages, expected_additional_imports",
        (
            (False, set()),
            (
                True,
                {
                    DirectImport(
                        importer=MODULE_FILE_UNMODIFIED.module,
                        imported=Module("external"),
                        line_number=100,
                        line_contents="import external",
                    ),
                },
            ),
        ),
    )
    def test_uses_cache_for_module_with_same_mtime(
        self, include_external_packages, expected_additional_imports
    ):
        cache = Cache.setup(
            file_system=self.FILE_SYSTEM,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=include_external_packages,
        )

        assert (
            cache.read_imports(self.MODULE_FILE_UNMODIFIED)
            == {
                DirectImport(
                    importer=self.MODULE_FILE_UNMODIFIED.module,
                    imported=Module("yellow"),
                    line_number=11,
                    line_contents="import yellow",
                ),
                DirectImport(
                    importer=self.MODULE_FILE_UNMODIFIED.module,
                    imported=Module("brown"),
                    line_number=22,
                    line_contents="import brown",
                ),
            }
            | expected_additional_imports
        )

    def test_raises_cache_miss_for_missing_module_from_data(self):
        file_system = FakeFileSystem(
            contents="""
                .grimp_cache/
                    mypackage.meta.json
                    mypackage.data.json
                /path/to/mypackage/
                    __init__.py
                    somemodule.py
            """,
            content_map={
                ".grimp_cache/mypackage.meta.json": f"""{{
                    "mypackage.somemodule": {self.SOME_MTIME}
                }}""",
                ".grimp_cache/mypackage.data.json": """{}""",
            },
        ).convert_to_basic()
        module_file = ModuleFile(module=Module("mypackage.somemodule"), mtime=self.SOME_MTIME)
        cache = Cache.setup(
            file_system=file_system,
            found_packages={
                FoundPackage(
                    name="mypackage",
                    directory="/path/to/mypackage/",
                    module_files=frozenset({module_file}),
                ),
            },
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        with pytest.raises(CacheMiss):
            cache.read_imports(module_file)

    @pytest.mark.parametrize("serialized_mtime", ("INVALID_JSON", '["wrong", "type"]'))
    def test_raises_cache_miss_for_corrupt_meta_file(self, serialized_mtime):
        file_system = FakeFileSystem(
            contents="""
                .grimp_cache/
                    mypackage.meta.json
                    mypackage.data.json
                /path/to/mypackage/
                    __init__.py
                    foo/
                        __init__.py
                        new.py
                        unmodified.py
                        modified.py
            """,
            content_map={
                ".grimp_cache/mypackage.meta.json": f"""{{
                    "mypackage.foo.modified": {serialized_mtime}
                }}""",
                ".grimp_cache/mypackage.data.json": """{
                    "mypackage.foo.modified": [
                        ["stale", 33, "We should not use this cached value."]
                    ]
                }""",
            },
        ).convert_to_basic()
        cache = Cache.setup(
            file_system=file_system,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        with pytest.raises(CacheMiss):
            cache.read_imports(self.MODULE_FILE_MODIFIED)

    @pytest.mark.parametrize("serialized_import", ("INVALID_JSON", '["wrong", "type"]'))
    def test_raises_cache_miss_for_corrupt_data_file(self, serialized_import):
        file_system = FakeFileSystem(
            contents="""
                .grimp_cache/
                    mypackage.meta.json
                    mypackage.data.json
                /path/to/mypackage/
                    __init__.py
                    foo/
                        __init__.py
                        new.py
                        unmodified.py
                        modified.py
            """,
            content_map={
                ".grimp_cache/mypackage.meta.json": f"""{{
                    "mypackage.foo.modified": {self.SOME_MTIME - 1}
                }}""",
                ".grimp_cache/mypackage.data.json": f"""{{
                    "mypackage.foo.modified": [
                        {serialized_import}
                    ]
                }}""",
            },
        ).convert_to_basic()
        cache = Cache.setup(
            file_system=file_system,
            found_packages=self.FOUND_PACKAGES,
            namer=SimplisticFileNamer,
            include_external_packages=False,
        )

        with pytest.raises(CacheMiss):
            cache.read_imports(self.MODULE_FILE_MODIFIED)

    @pytest.mark.parametrize(
        "include_external_packages, expected_additional_imports",
        (
            (False, set()),
            (
                True,
                {
                    DirectImport(
                        importer=Module("anotherpackage.unmodified"),
                        imported=Module("anotherexternal"),
                        line_number=100,
                        line_contents="import anotherexternal",
                    ),
                },
            ),
        ),
    )
    def test_uses_cache_multiple_packages(
        self, include_external_packages, expected_additional_imports
    ):
        module_file = ModuleFile(module=Module("anotherpackage.unmodified"), mtime=self.SOME_MTIME)
        cache = Cache.setup(
            file_system=self.FILE_SYSTEM,
            found_packages=self.FOUND_PACKAGES
            | {
                FoundPackage(
                    name="anotherpackage",
                    directory="/path/to/anotherpackage/",
                    module_files=frozenset({module_file}),
                ),
            },
            namer=SimplisticFileNamer,
            include_external_packages=include_external_packages,
        )

        result = cache.read_imports(module_file)

        assert (
            result
            == {
                DirectImport(
                    importer=module_file.module,
                    imported=Module("purple"),
                    line_number=11,
                    line_contents="import purple",
                ),
                DirectImport(
                    importer=module_file.module,
                    imported=Module("green"),
                    line_number=22,
                    line_contents="import green",
                ),
            }
            | expected_additional_imports
        )

    @pytest.mark.parametrize("cache_dir", ("/tmp/some-cache-dir", "/tmp/some-cache-dir/", None))
    @pytest.mark.parametrize(
        "include_external_packages, expected_data_file_name",
        (
            (False, "blue,green.data.json"),
            (True, "blue,green:external.data.json"),
        ),
    )
    def test_write_to_cache(
        self, include_external_packages, expected_data_file_name, cache_dir, caplog
    ):
        caplog.set_level(logging.INFO, logger=Cache.__module__)
        file_system = FakeFileSystem().convert_to_basic()
        blue_one = Module(name="blue.one")
        blue_two = Module(name="blue.two")
        green_one = Module(name="green.one")
        green_two = Module(name="green.two")
        mtimes = {
            blue_one: 10000.1,
            blue_two: 20000.2,
            green_one: 30000.3,
            green_two: 40000.4,
        }
        cache = Cache.setup(
            file_system=file_system,
            cache_dir=cache_dir,
            found_packages={
                FoundPackage(
                    name="blue",
                    module_files=frozenset(
                        {
                            ModuleFile(module=blue_one, mtime=mtimes[blue_one]),
                            ModuleFile(module=blue_two, mtime=mtimes[blue_two]),
                        }
                    ),
                    directory="-",
                ),
                FoundPackage(
                    name="green",
                    module_files=frozenset(
                        {
                            ModuleFile(module=green_one, mtime=mtimes[green_one]),
                            ModuleFile(module=green_two, mtime=mtimes[green_two]),
                        }
                    ),
                    directory="-",
                ),
            },
            include_external_packages=include_external_packages,
            namer=SimplisticFileNamer,
        )

        cache.write(
            imports_by_module={
                blue_one: {
                    DirectImport(
                        importer=blue_one,
                        imported=blue_two,
                        line_number=11,
                        line_contents="from . import two",
                    ),
                    DirectImport(
                        importer=blue_one,
                        imported=Module("externalpackage"),
                        line_number=22,
                        line_contents="import externalpackage",
                    ),
                },
                blue_two: set(),
                green_one: set(),
                green_two: {
                    DirectImport(
                        importer=green_two,
                        imported=green_one,
                        line_number=33,
                        line_contents="from . import one",
                    ),
                },
            },
        )

        # Assert the cache is written afterwards.
        expected_cache_dir = cache_dir.rstrip(file_system.sep) if cache_dir else ".grimp_cache"
        assert set(caplog.messages) == {
            f"No cache file: {expected_cache_dir}/{expected_data_file_name}.",
            f"No cache file: {expected_cache_dir}/blue.meta.json.",
            f"No cache file: {expected_cache_dir}/green.meta.json.",
            f"Wrote data cache file {expected_cache_dir}/{expected_data_file_name}.",
            f"Wrote meta cache file {expected_cache_dir}/blue.meta.json.",
            f"Wrote meta cache file {expected_cache_dir}/green.meta.json.",
        }
        expected = {
            f"{expected_cache_dir}/blue.meta.json": {
                blue_one.name: mtimes[blue_one],
                blue_two.name: mtimes[blue_two],
            },
            f"{expected_cache_dir}/green.meta.json": {
                green_one.name: mtimes[green_one],
                green_two.name: mtimes[green_two],
            },
            f"{expected_cache_dir}/{expected_data_file_name}": {
                blue_one.name: {
                    (blue_two.name, 11, "from . import two"),
                    ("externalpackage", 22, "import externalpackage"),
                },
                blue_two.name: set(),
                green_one.name: set(),
                green_two.name: {
                    (green_one.name, 33, "from . import one"),
                },
            },
        }
        for filename, expected_deserialized in expected.items():
            serialized = file_system.read(filename)
            deserialized = _unflake(json.loads(serialized))
            assert deserialized == expected_deserialized

    def test_write_to_cache_adds_marker_files(self):
        some_cache_dir = "/tmp/some-cache-dir"
        file_system = FakeFileSystem().convert_to_basic()
        cache = Cache.setup(
            file_system=file_system,
            cache_dir=some_cache_dir,
            found_packages=set(),
            include_external_packages=False,  # Value shouldn't matter.
            namer=SimplisticFileNamer,
        )

        cache.write(
            imports_by_module={},
        )

        assert file_system.read(f"{some_cache_dir}/.gitignore") == (
            "# Automatically created by Grimp.\n" "*"
        )
        assert file_system.read(f"{some_cache_dir}/CACHEDIR.TAG") == (
            "Signature: 8a477f597d28d172789f06886806bc55\n"
            "# This file is a cache directory tag automatically created by Grimp.\n"
            "# For information about cache directory tags see https://bford.info/cachedir/"
        )


def _unflake(flakey_data):
    # Takes the deserialized data that could come in any
    # order and order it consistently, so the test isn't flakey.
    non_flakey_data = {}
    for key, value in flakey_data.items():
        if isinstance(value, list):
            non_flakey_value = {tuple(i) for i in value}
        else:
            non_flakey_value = value
        non_flakey_data[key] = non_flakey_value
    return non_flakey_data
