import abc
from typing import Dict, Set

from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.domain.valueobjects import DirectImport, Module


class AbstractImportScanner(abc.ABC):
    """
    Statically analyses some Python modules for import statements within their shared package.
    """

    def __init__(
        self,
        modules_by_package_directory: Dict[str, Set[Module]],
        file_system: AbstractFileSystem,
        include_external_packages: bool = False,
    ) -> None:
        """
        Args:
            - modules_by_package_directory: Dictionary containing all the modules for analysis,
                                            keyed by the full file path of the directory of the
                                            root package for each set of modules. For example:
                                            {
                                                "/path/to/packageone": {
                                                    Module("packageone"),
                                                    Module("packageone.foo"),
                                                    Module("packageone.bar"),
                                                    Module("packageone.bar.alpha"),
                                                },
                                                "/path/to/packagetwo": {
                                                    Module("packagetwo"),
                                                    Module("packagetwo.baz"),
                                                    ...
                                                },
                                            }
            - file_system:                  The file system interface to use.
            - include_external_packages:    Whether to include imports of external modules (i.e.
                                            modules not contained in modules_by_package_directory)
                                            in the results.
        """
        self.modules_by_package_directory = modules_by_package_directory
        self.file_system = file_system
        self.include_external_packages = include_external_packages

        # Flatten all the modules into a set.
        self.modules: Set[Module] = set()
        for package_modules in self.modules_by_package_directory.values():
            self.modules |= package_modules

    @abc.abstractmethod
    def scan_for_imports(self, module: Module) -> Set[DirectImport]:
        """
        Statically analyses the given module and returns an iterable of Modules that
        it imports.
        """
        raise NotImplementedError
