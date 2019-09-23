import abc

from .filesystem import AbstractFileSystem


class AbstractPackageFinder(abc.ABC):
    @abc.abstractmethod
    def determine_package_directory(
        self, package_name: str, file_system: AbstractFileSystem
    ) -> str:
        raise NotImplementedError
