import abc
from typing import Set

from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.domain.valueobjects import Module, DirectImport


class AbstractImportScanner(abc.ABC):
    """
    Statically analyses some Python modules for import statements within their shared package.
    """
    def __init__(self,
                 modules: Set[Module],
                 package_directory: str,
                 file_system: AbstractFileSystem,
                 include_external_packages: bool = False) -> None:
        """
        Args:
            - modules:           All the modules in the package we are scanning.
            - package_directory: The full file path of the directory of the package that
                                 contains all the modules are in, for example '/path/to/mypackage'.
            - file_system:       The file system interface to use.
            - include_external_packages: Whether to include imports of external packages
                                         in the results.
        """
        self.modules = modules
        self.package_directory = package_directory
        self.file_system = file_system
        self.include_external_packages = include_external_packages

    @abc.abstractmethod
    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        """
        Statically analyses the given module and returns an iterable of Modules that
        it imports.
        """
        raise NotImplementedError
