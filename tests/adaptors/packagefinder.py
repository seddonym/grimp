from grimp.application.ports.packagefinder import AbstractPackageFinder
from grimp.application.ports.filesystem import AbstractFileSystem


class BaseFakePackageFinder(AbstractPackageFinder):
    directory_map: dict[str, str] = {}

    def determine_package_directories(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> set[str]:
        return {self.directory_map[package_name]}
