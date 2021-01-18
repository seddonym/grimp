import importlib.util
import logging
import sys

from grimp import exceptions
from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.application.ports.packagefinder import AbstractPackageFinder

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
                "Could not find package '{}' in your Python path.".format(package_name)
            )

        if package_filename.has_location and package_filename.origin:
            return file_system.dirname(package_filename.origin)

        raise exceptions.NamespacePackageEncountered(
            f"Package {package_name} appears to be a 'namespace package' (see PEP 420), "
            "which is not currently supported. If this is not deliberate, adding an __init__.py "
            "file should fix the problem."
        )
