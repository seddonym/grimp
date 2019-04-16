import importlib.util
import sys
import logging

from grimp.application.ports.packagefinder import AbstractPackageFinder
from grimp.application.ports.filesystem import AbstractFileSystem


logger = logging.getLogger(__name__)


class ImportLibPackageFinder(AbstractPackageFinder):
    def determine_package_directory(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> str:
        # TODO - do we need to add the current working directory here?
        # Attempt to locate the package file.
        package_filename = importlib.util.find_spec(package_name)
        if not package_filename:
            logger.debug("sys.path: {}".format(sys.path))
            raise ValueError(
                "Could not find package '{}' in your Python path.".format(package_name))
        assert package_filename.origin  # For type checker.
        return file_system.dirname(package_filename.origin)
