from typing import Set, Protocol

from grimp.application.ports.filesystem import BasicFileSystem
from grimp.application.ports.modulefinder import FoundPackage
from grimp.domain.valueobjects import DirectImport, Module


class AbstractImportScanner(Protocol):
    """
    Statically analyses some Python modules for import statements within their shared package.
    """

    def __init__(
        self,
        file_system: BasicFileSystem,
        found_packages: Set[FoundPackage],
        include_external_packages: bool = False,
    ) -> None:
        """
        Args:
            - found_packages:                Set of FoundPackages containing all the modules
                                             for analysis.
            - file_system:                   The file system interface to use.
            - include_external_packages:     Whether to include imports of external modules (i.e.
                                             modules not contained in modules_by_package_directory)
                                             in the results.
        """

    def scan_for_imports(
        self, module: Module, *, exclude_type_checking_imports: bool = False
    ) -> Set[DirectImport]:
        """
        Statically analyses the given module and returns an iterable of Modules that
        it imports.
        """
