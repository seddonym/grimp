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
    def determine_package_directories(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> set[str]:
        # Attempt to locate the package file.
        spec = importlib.util.find_spec(package_name)
        if not spec:
            logger.debug(f"sys.path: {sys.path}")
            raise ValueError(f"Could not find package '{package_name}' in your Python path.")

        if spec.has_location and spec.origin:
            if not self._is_a_package(spec, file_system) or self._has_a_non_namespace_parent(spec):
                raise exceptions.NotATopLevelModule

        assert spec.submodule_search_locations  # This should be the case if spec.has_location.
        return set(spec.submodule_search_locations)

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
