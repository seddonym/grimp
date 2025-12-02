import abc

from .filesystem import AbstractFileSystem


class AbstractPackageFinder(abc.ABC):
    @abc.abstractmethod
    def determine_package_directories(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> set[str]:
        raise NotImplementedError
