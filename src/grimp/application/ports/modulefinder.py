import abc
from dataclasses import dataclass
from typing import FrozenSet

from grimp.domain.valueobjects import Module

from .filesystem import AbstractFileSystem


@dataclass(frozen=True)
class ModuleFile:
    module: Module
    mtime: float


@dataclass(frozen=True)
class FoundPackage:
    """
    Set of modules found under a single package, together with metadata.
    """

    name: str
    directory: str
    module_files: FrozenSet[ModuleFile]


class AbstractModuleFinder(abc.ABC):
    """
    Finds Python modules inside a package.
    """

    @abc.abstractmethod
    def find_package(
        self, package_name: str, package_directory: str, file_system: AbstractFileSystem
    ) -> FoundPackage:
        """
        Searches the package for all importable Python modules.
        """
        raise NotImplementedError
