from grimp.application.ports.packagefinder import AbstractPackageFinder
from grimp.application.ports.filesystem import AbstractFileSystem


class BaseFakePackageFinder(AbstractPackageFinder):
    directory_map = {}

    def determine_package_directory(
            self, package_name: str, file_system: AbstractFileSystem
    ) -> str:
        return self.directory_map[package_name]
