import logging
from typing import Iterable, List

from grimp.application.ports import modulefinder
from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.domain.valueobjects import Module

logger = logging.getLogger(__name__)


class ModuleFinder(modulefinder.AbstractModuleFinder):
    def find_package(
        self, package_name: str, package_directory: str, file_system: AbstractFileSystem
    ) -> modulefinder.FoundPackage:
        self.file_system = file_system

        module_files: List[modulefinder.ModuleFile] = []

        for module_filename in self._get_python_files_inside_package(package_directory):
            module_name = self._module_name_from_filename(
                package_name, module_filename, package_directory
            )
            module_mtime = self.file_system.get_mtime(module_filename)
            module_files.append(
                modulefinder.ModuleFile(module=Module(module_name), mtime=module_mtime)
            )

        return modulefinder.FoundPackage(
            name=package_name,
            directory=package_directory,
            module_files=frozenset(module_files),
        )

    def _get_python_files_inside_package(self, directory: str) -> Iterable[str]:
        """
        Get a list of Python files within the supplied package directory.
         Return:
            Generator of Python file names.
        """
        for dirpath, dirs, files in self.file_system.walk(directory):
            # Don't include directories that aren't Python packages,
            # nor their subdirectories.
            if "__init__.py" not in files:
                for d in list(dirs):
                    dirs.remove(d)
                continue

            # Don't include hidden directories.
            dirs_to_remove = [d for d in dirs if self._should_ignore_dir(d)]
            for d in dirs_to_remove:
                dirs.remove(d)

            for filename in files:
                if self._is_python_file(filename, dirpath):
                    yield self.file_system.join(dirpath, filename)

    def _should_ignore_dir(self, directory: str) -> bool:
        # TODO: make this configurable.
        # Skip adding directories that are hidden.
        return directory.startswith(".")

    def _is_python_file(self, filename: str, dirpath: str) -> bool:
        """
        Given a filename, return whether it's a Python file.

        Files with extra dots in the name won't be treated as Python files.

        Args:
            filename (str): the filename, excluding the path.
        Returns:
            bool: whether it's a Python file.
        """
        # Ignore hidden files.
        if filename.startswith("."):
            return False

        if not filename.endswith(".py"):
            return False

        # Ignore files like some.module.py.
        if filename.count(".") > 1:
            logger.warning(
                "Warning: skipping module with too many dots in the name: "
                f"{dirpath}{self.file_system.sep}{filename}"
            )
            return False

        return True

    def _module_name_from_filename(
        self, package_name: str, filename_and_path: str, package_directory: str
    ) -> str:
        """
        Args:
            package_name (string) - the importable name of the top level package. Could
                be namespaced.
            filename_and_path (string) - the full name of the Python file.
            package_directory (string) - the full path of the top level Python package directory.
         Returns:
            Absolute module name for importing (string).
        """
        internal_filename_and_path = filename_and_path[len(package_directory) :]
        internal_filename_and_path_without_extension = internal_filename_and_path[1:-3]
        components = [package_name] + internal_filename_and_path_without_extension.split(
            self.file_system.sep
        )
        if components[-1] == "__init__":
            components.pop()
        return ".".join(components)
