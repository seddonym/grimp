from copy import copy

import pytest  # type: ignore

from tests.adaptors.filesystem import FakeFileSystem


class TestFakeFileSystem:
    def test_walk(self):
        file_system = FakeFileSystem("""
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
        """)
        assert [
            ('/path/to/mypackage', ['foo'], ['__init__.py']),
            ('/path/to/mypackage/foo', ['two'], ['__init__.py', 'one.py']),
            ('/path/to/mypackage/foo/two', [], ['__init__.py', 'green.py', 'blue.py']),
        ] == list(file_system.walk('/path/to/mypackage'))

    def test_empty_if_directory_does_not_exist(self):
        file_system = FakeFileSystem("""
            /path/to/mypackage/
                __init__.py
        """)
        assert [] == list(file_system.walk('/path/to/nonexistent/package'))

    def test_dirname(self):
        file_system = FakeFileSystem()
        assert '/path/to' == file_system.dirname('/path/to/file.txt')

    def test_join(self):
        file_system = FakeFileSystem()
        assert '/path/to/mypackage/file.py' == file_system.join('/path/to', 'mypackage', 'file.py')

    def test_split(self):
        file_system = FakeFileSystem()
        assert ('/path/to/mypackage', 'file.py') == file_system.split(
            '/path/to/mypackage/file.py')

    def test_dirnames_can_be_modified_in_place(self):
        """
        From the os.walk docs:
            The caller can modify the dirnames list in-place (perhaps using del or slice
            assignment), and walk() will only recurse into the subdirectories whose names
            remain in dirnames; this can be used to prune the search, impose a specific order
            of visiting, or even to inform walk() about directories the caller creates or renames
            before it resumes walk() again.
        """
        file_system = FakeFileSystem("""
                    /path/to/mypackage/
                        foo/
                            one.txt
                            skipme/
                                two.txt
                            dontskip/
                                three.txt
                        bar/
                            four.txt
        """)

        expected_tuples = [
            ('/path/to/mypackage', ['foo', 'bar'], []),
            ('/path/to/mypackage/foo', ['skipme', 'dontskip'], ['one.txt']),
            ('/path/to/mypackage/foo/dontskip', [], ['three.txt']),
            ('/path/to/mypackage/bar', [], ['four.txt']),
        ]

        actual_tuples = []
        for dirpath, dirs, files in file_system.walk('/path/to/mypackage'):
            # Ensure we make a copy of dirs (since we change it).
            actual_tuples.append((dirpath, copy(dirs), files))
            if 'skipme' in dirs:
                dirs.remove('skipme')
                continue

        assert expected_tuples == actual_tuples

    @pytest.mark.parametrize(
        'file_name, expected_contents', (
            ('/path/to/mypackage/readme.txt', 'Lorem'),
            ('/path/to/mypackage/foo/one.txt', 'Ipsum'),
            # Listed in contents, but not in content_map.
            ('/path/to/mypackage/foo/two/green.txt', ''),
            # Listed in content_map, but not in contents.
            ('/path/to/mypackage/foo/two/blue.txt', 'Dolor sic'),
            ('/path/to/mypackage/nonexistent', FileNotFoundError),
            ('/path/to/mypackage/foo/three/nonexistent', FileNotFoundError),
            # TODO - should we raise an exception if we attempt to read a directory?
        )
    )
    def test_read(self, file_name, expected_contents):
        file_system = FakeFileSystem(
            contents="""
            /path/to/mypackage/
                readme.txt
                foo/
                    one.txt
                    two/
                        green.txt
            """,
            content_map={
                '/path/to/mypackage/readme.txt': 'Lorem',
                '/path/to/mypackage/foo/one.txt': 'Ipsum',
                '/path/to/mypackage/foo/two/blue.txt': 'Dolor sic',
            }
        )
        if isinstance(expected_contents, type) and issubclass(expected_contents, Exception):
            with pytest.raises(expected_contents):
                file_system.read(file_name)
        else:
            assert expected_contents == file_system.read(file_name)
