from typing import TypeAlias
from copy import copy
import pytest  # type: ignore
from grimp.application.ports.filesystem import BasicFileSystem
from tests.adaptors.filesystem import FakeFileSystem
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]


class _Base:
    """
    Tests for methods that AbstractFileSystem and BasicFileSystem share.
    """

    file_system_cls: type[BasicFileSystem]

    @pytest.mark.parametrize("path", ("/path/to", "/path/to/"))
    def test_join(self, path):
        file_system = self.file_system_cls()
        assert "/path/to/mypackage/file.py" == file_system.join(path, "mypackage", "file.py")

    @pytest.mark.parametrize(
        "path, expected",
        [
            ("", ("", "")),
            ("/path", ("/", "path")),
            ("some-file", ("", "some-file")),
            ("/path/to/mypackage/", ("/path/to/mypackage", "")),
            ("/path/to/mypackage/some-file", ("/path/to/mypackage", "some-file")),
            ("/path/to/mypackage/some-file.py", ("/path/to/mypackage", "some-file.py")),
        ],
    )
    def test_split(self, path, expected):
        file_system = self.file_system_cls()
        assert file_system.split(path) == expected

    @pytest.mark.parametrize(
        "file_name, expected",
        [
            ("/path/to/mypackage", False),
            ("/path/to/mypackage/", False),
            ("/path/to/mypackage/readme.txt", True),
            ("/path/to/mypackage/foo/one.txt", True),
            ("/path/to/mypackage/foo/two/green.txt", True),
            ("/path/to/mypackage/bar/blue.txt", True),
            ("/path/to/nonexistent.txt", False),
            ("/path/to/mypackage/purple.txt", False),
        ],
    )
    def test_exists_content_only(self, file_name, expected):
        file_system = self.file_system_cls(
            contents="""
            /path/to/mypackage/
                readme.txt
                foo/
                    one.txt
                    two/
                        green.txt
                bar/
                    blue.txt
            """
        )

        assert file_system.exists(file_name) == expected

    @pytest.mark.parametrize(
        "file_name, expected",
        [
            ("/path/to/", False),
            ("/path/to/file.txt", True),
            ("/path/to/a/deeper/file.txt", True),
            ("/path/to/nonexistent.txt", False),
        ],
    )
    def test_exists_content_map_only(self, file_name, expected):
        file_system = self.file_system_cls(
            content_map={
                "/path/to/file.txt": "hello",
                "/path/to/a/deeper/file.txt": "hello",
            }
        )

        assert file_system.exists(file_name) == expected

    @pytest.mark.parametrize(
        "file_name, expected",
        [
            ("/path/to/file.txt", True),
            ("/path/to/mypackage/foo/two/green.txt", True),
            ("/path/to/nonexistent.txt", False),
        ],
    )
    def test_exists_content_map_and_content(self, file_name, expected):
        file_system = self.file_system_cls(
            contents="""
            /path/to/mypackage/
                readme.txt
                foo/
                    one.txt
                    two/
                        green.txt
            """,
            content_map={
                "/path/to/file.txt": "hello",
                "/path/to/a/deeper/file.txt": "hello",
            },
        )

        assert file_system.exists(file_name) == expected

    @pytest.mark.parametrize(
        "file_name, expected_contents",
        (
            ("/path/to/mypackage/readme.txt", "Lorem"),
            ("/path/to/mypackage/foo/one.txt", "Ipsum"),
            # Listed in contents, but not in content_map.
            ("/path/to/mypackage/foo/two/green.txt", ""),
            # Listed in content_map, but not in contents.
            ("/path/to/mypackage/foo/two/blue.txt", "Dolor sic"),
            (
                "/path/to/mypackage/indented.txt",
                "This is indented text.\n    We should dedent it.",
            ),
            ("/path/to/mypackage/nonexistent", FileNotFoundError),
            ("/path/to/mypackage/foo/three/nonexistent", FileNotFoundError),
            # TODO - should we raise an exception if we attempt to read a directory?
        ),
    )
    def test_read(self, file_name, expected_contents):
        file_system = self.file_system_cls(
            contents="""
            /path/to/mypackage/
                readme.txt
                foo/
                    one.txt
                    two/
                        green.txt
            """,
            content_map={
                "/path/to/mypackage/readme.txt": "Lorem",
                "/path/to/mypackage/foo/one.txt": "Ipsum",
                "/path/to/mypackage/foo/two/blue.txt": "Dolor sic",
                "/path/to/mypackage/indented.txt": """
                This is indented text.
                    We should dedent it.
                """,
            },
        )
        if isinstance(expected_contents, type) and issubclass(expected_contents, Exception):
            with pytest.raises(expected_contents):
                file_system.read(file_name)
        else:
            assert file_system.read(file_name) == expected_contents

    def test_write(self):
        some_filename, some_contents = "path/to/some-file.txt", "Some contents."
        file_system = self.file_system_cls()

        file_system.write(some_filename, some_contents)

        assert file_system.read(some_filename) == some_contents


WalkReturn: TypeAlias = tuple[str, list[str], list[str]]


class TestFakeFileSystem(_Base):
    file_system_cls = FakeFileSystem

    MYPACKAGE: WalkReturn = ("/path/to/mypackage", ["foo"], ["__init__.py"])
    MYPACKAGE_FOO: WalkReturn = ("/path/to/mypackage/foo", ["two"], ["__init__.py", "one.py"])
    MYPACKAGE_FOO_TWO: WalkReturn = (
        "/path/to/mypackage/foo/two",
        [],
        ["__init__.py", "green.py", "blue.py"],
    )

    @pytest.mark.parametrize(
        "directory, expected",
        [
            (
                "/path/to/mypackage",
                [MYPACKAGE, MYPACKAGE_FOO, MYPACKAGE_FOO_TWO],
            ),
        ],
    )
    def test_walk(self, directory: str, expected: list[WalkReturn]):
        file_system = self.file_system_cls(
            """
            /path/to/mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
                    two/
                        __init__.py
                        green.py
                        blue.py
            /anotherpackage/
                another.txt
        """
        )

        result = list(file_system.walk(directory))

        assert result == expected

    def test_empty_if_directory_does_not_exist(self):
        file_system = self.file_system_cls(
            """
            /path/to/mypackage/
                __init__.py
        """
        )
        assert [] == list(file_system.walk("/path/to/nonexistent/package"))

    def test_dirname(self):
        file_system = self.file_system_cls()
        assert "/path/to" == file_system.dirname("/path/to/file.txt")

    def test_dirnames_can_be_modified_in_place(self):
        """
        From the os.walk docs:
            The caller can modify the dirnames list in-place (perhaps using del or slice
            assignment), and walk() will only recurse into the subdirectories whose names
            remain in dirnames; this can be used to prune the search, impose a specific order
            of visiting, or even to inform walk() about directories the caller creates or renames
            before it resumes walk() again.
        """
        file_system = self.file_system_cls(
            """
            /path/to/mypackage/
                foo/
                    one.txt
                    skipme/
                        two.txt
                    dontskip/
                        three.txt
                bar/
                    four.txt
        """
        )

        expected_tuples = [
            ("/path/to/mypackage", ["foo", "bar"], []),
            ("/path/to/mypackage/foo", ["skipme", "dontskip"], ["one.txt"]),
            ("/path/to/mypackage/foo/dontskip", [], ["three.txt"]),
            ("/path/to/mypackage/bar", [], ["four.txt"]),
        ]

        actual_tuples = []
        for dirpath, dirs, files in file_system.walk("/path/to/mypackage"):
            # Ensure we make a copy of dirs (since we change it).
            actual_tuples.append((dirpath, copy(dirs), files))
            if "skipme" in dirs:
                dirs.remove("skipme")
                continue

        assert expected_tuples == actual_tuples


class TestFakeBasicFileSystem(_Base):
    file_system_cls = rust.FakeBasicFileSystem
