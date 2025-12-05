from typing import Any
from collections.abc import Generator

import yaml

from grimp.application.ports.filesystem import AbstractFileSystem, BasicFileSystem
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

DEFAULT_MTIME = 10000.0


class FakeFileSystem(AbstractFileSystem):
    def __init__(
        self,
        contents: str | None = None,
        content_map: dict[str, str] | None = None,
        mtime_map: dict[str, float] | None = None,
    ) -> None:
        """
        Files can be declared as existing in the file system in two different ways, either
        in a contents string (which is a quick way of defining a lot of files), or in content_map
        (which specifies the actual contents of a file in the file system). For a file to be
        treated as existing, it needs to be declared in at least one of these. If it isn't
        declared in content_map, the file will behave as an empty file.

        Args:
            contents: a string in the following format:

                /path/to/mypackage/
                    __init__.py
                    foo/
                        __init__.py
                        one.py
                        two/
                            __init__.py
                            green.py
                            blue.py

            content_map: A dictionary keyed with filenames, with values that are the contents.
                         If present in content_map, .read(filename) will return the string.
                {
                    '/path/to/foo/__init__.py': "from . import one",
                }
            mtime_map: A dictionary keyed with filenames, with values that are the mtimes
                       i.e. last modified times.
        """
        self.contents = self._parse_contents(contents)
        self._raw_contents = contents
        self.content_map = content_map if content_map else {}
        self.mtime_map: dict[str, float] = mtime_map if mtime_map else {}

    @property
    def sep(self) -> str:
        return "/"

    def dirname(self, filename: str) -> str:
        """
        Return the full path to the directory name of the supplied filename.

        E.g. '/path/to/filename.py' will return '/path/to'.
        """
        return self.split(filename)[0]

    def walk(self, directory_name):
        """
        Given a directory, walk the file system recursively.

        For each directory in the tree rooted at directory top (including top itself),
        it yields a 3-tuple (dirpath, dirnames, filenames).
        """
        # Navigate through the nested structure to find the directory
        directory_components = [c for c in directory_name.split("/") if c]

        contents = self.contents
        for component in directory_components:
            key_with_prefix = "/" + component
            if key_with_prefix in contents:
                contents = contents[key_with_prefix]
            elif component in contents:
                contents = contents[component]
            else:
                return []

        # If we found a file (None) instead of a directory, return empty
        if contents is None:
            return []

        yield from self._walk_contents(contents, containing_directory=directory_name)

    def _walk_contents(
        self, directory_contents: dict[str, Any], containing_directory: str
    ) -> Generator[tuple[str, list[str], list[str]], None, None]:
        directories = []
        files = []
        for key, value in directory_contents.items():
            if value is None:
                files.append(key)
            else:
                directories.append(key)

        yield (containing_directory, directories, files)

        if directories:
            for directory in directories:
                yield from self._walk_contents(
                    directory_contents=directory_contents[directory],
                    containing_directory=self.join(containing_directory, directory),
                )

    def join(self, *components: str) -> str:
        return self.sep.join(c.rstrip(self.sep) for c in components)

    def split(self, file_name: str) -> tuple[str, str]:
        components = file_name.split(self.sep)
        if len(components) == 2:
            # Handle case where file is child of the root, i.e. /some-file.txt.
            # In this case, to conform with the interface we ensure the leading slash
            # is included in the returned head.
            components.insert(0, "")
        return (self.sep.join(components[:-1]), components[-1])

    def _parse_contents(self, raw_contents: str | None) -> dict[str, Any]:
        """
        Returns the raw contents parsed in the form:
            {
                "/path": {
                    "/to":
                        "/mypackage": {
                            "__init__.py": None,
                            "foo": {
                                "__init__.py": None,
                                "one.py": None,
                                "two": {
                                    "__init__.py": None,
                                    "blue.py": None,
                                    "green.py": None,
                                }
                            },
                        },
                    },
                }
            }
        """
        if raw_contents is None:
            return {}

        raw_lines = [line for line in raw_contents.split("\n") if line.strip()]
        dedented_lines = self._dedent(raw_lines)

        # Group lines by their root paths
        # A root path is a line that starts with "/" and has no indentation
        root_path_groups = []
        current_group: list[str] = []

        for line in dedented_lines:
            if line.startswith("/") and not line.startswith("    "):
                # This is a new root path
                if current_group:
                    root_path_groups.append(current_group)
                current_group = [line]
            else:
                current_group.append(line)

        if current_group:
            root_path_groups.append(current_group)

        # Process each root path group
        result: dict[str, Any] = {}
        for group in root_path_groups:
            # First line is the root path
            root_path = group[0].rstrip().rstrip("/")
            path_components = [c for c in root_path.split("/") if c]

            # Remaining lines are the file tree
            yamlified_lines = []
            for line in group[1:]:
                trimmed_line = line.rstrip().rstrip("/")
                yamlified_line = trimmed_line + ":"
                yamlified_lines.append(yamlified_line)

            yamlified_string = "\n".join(yamlified_lines)
            nested_contents = yaml.safe_load(yamlified_string) if yamlified_lines else {}

            # Build the nested structure from path components
            group_result = nested_contents
            for component in reversed(path_components):
                group_result = {"/" + component: group_result}

            # Merge into result
            self._deep_merge(result, group_result)

        return result

    def _deep_merge(self, target: dict, source: dict) -> None:
        """Merge source dict into target dict recursively."""
        for key, value in source.items():
            if key in target and isinstance(target[key], dict) and isinstance(value, dict):
                self._deep_merge(target[key], value)
            else:
                target[key] = value

    def _dedent(self, lines: list[str]) -> list[str]:
        """
        Dedent all lines by the same amount.
        """
        first_line = lines[0]
        first_line_indent = len(first_line) - len(first_line.lstrip())
        return [line[first_line_indent:] for line in lines]

    def read(self, file_name: str) -> str:
        if not self.exists(file_name):
            raise FileNotFoundError
        try:
            file_contents = self.content_map[file_name]
        except KeyError:
            return ""
        raw_lines = [line for line in file_contents.split("\n") if line.strip()]
        dedented_lines = self._dedent(raw_lines)
        return "\n".join(dedented_lines)

    def exists(self, file_name: str) -> bool:
        # The file should exist if it's either declared in contents or in content_map.
        if file_name in self.content_map.keys():
            return True

        # Split the file path into components and navigate through nested structure
        file_components = [c for c in file_name.split("/") if c]

        contents = self.contents
        for i, component in enumerate(file_components):
            # First, try with "/" prefix (for the top-level path structure)
            key_with_prefix = "/" + component
            if key_with_prefix in contents:
                contents = contents[key_with_prefix]
            # If not found, try without prefix (for files and directories in the tree)
            elif component in contents:
                contents = contents[component]
            else:
                return False

            # If we've reached None, it's a file (leaf node)
            if contents is None:
                return True

        # If we've navigated through all components and haven't hit None,
        # it's a directory, not a file - return False
        return False

    def get_mtime(self, file_name: str) -> float:
        if not self.exists(file_name):
            raise FileNotFoundError(f"{file_name} does not exist.")
        return self.mtime_map.get(file_name, DEFAULT_MTIME)

    def write(self, file_name: str, contents: str) -> None:
        self.content_map[file_name] = contents
        self.mtime_map[file_name] = DEFAULT_MTIME

    def convert_to_basic(self) -> BasicFileSystem:
        """
        Convert this file system to a BasicFileSystem.
        """
        return rust.FakeBasicFileSystem(
            contents=self._raw_contents,
            content_map=self.content_map,
        )
