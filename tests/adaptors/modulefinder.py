from grimp.application.ports.modulefinder import AbstractModuleFinder, FoundPackage, ModuleFile
from grimp.application.ports.filesystem import AbstractFileSystem


class BaseFakeModuleFinder(AbstractModuleFinder):
    module_files_by_package_name: dict[str, frozenset[ModuleFile]] = {}

    def find_package(
        self, package_name: str, package_directory: str, file_system: AbstractFileSystem
    ) -> FoundPackage:
        return FoundPackage(
            name=package_name,
            directory=package_directory,
            module_files=self.module_files_by_package_name[package_name],
        )
