import importlib.util
import logging
import sys
from importlib.machinery import ModuleSpec

from grimp import exceptions
from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.application.ports.packagefinder import AbstractPackageFinder
from grimp.domain.valueobjects import Module

logger = logging.getLogger(__name__)


class ImportLibPackageFinder(AbstractPackageFinder):
    def determine_package_directory(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> str:
        # TODO - do we need to add the current working directory here?
        # Attempt to locate the package file.
        spec = importlib.util.find_spec(package_name)
        if not spec:
            logger.debug("sys.path: {}".format(sys.path))
            raise ValueError(
                "Could not find package '{}' in your Python path.".format(package_name)
            )

        if spec.has_location and spec.origin:
            if not self._is_a_package(spec, file_system) or self._has_a_non_namespace_parent(spec):
                raise exceptions.NotATopLevelModule

            return file_system.dirname(spec.origin)

        raise exceptions.NamespacePackageEncountered(
            f"Package '{package_name}' is a namespace package (see PEP 420). Try specifying the "
            "portion name instead. If you are not intentionally using namespace packages, "
            "adding an __init__.py file should fix the problem."
        )

    def _is_a_package(self, spec: ModuleSpec, file_system: AbstractFileSystem) -> bool:
        assert spec.origin
        filename = file_system.split(spec.origin)[1]
        return filename == "__init__.py"

    def _has_a_non_namespace_parent(self, spec: ModuleSpec) -> bool:
        module = Module(spec.name)

        if module.root == module:
            # The module has no parent.
            return False

        root_spec = importlib.util.find_spec(module.parent.name)
        assert root_spec
        return root_spec.has_location
