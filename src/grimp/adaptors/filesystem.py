import os
import tokenize
from typing import Iterator, List, Tuple

from grimp.application.ports.filesystem import AbstractFileSystem


class FileSystem(AbstractFileSystem):
    """
    Abstraction around file system calls.
    """

    @property
    def sep(self) -> str:
        return os.sep

    def dirname(self, filename: str) -> str:
        return os.path.dirname(filename)

    def walk(self, directory_name: str) -> Iterator[Tuple[str, List[str], List[str]]]:
        yield from os.walk(directory_name)

    def join(self, *components: str) -> str:
        return os.path.join(*components)

    def split(self, file_name: str) -> Tuple[str, str]:
        return os.path.split(file_name)

    def read(self, file_name: str) -> str:
        # Use tokenize.open to give us a better chance of successfully decoding
        # source code in a non-ascii compatible encoding.
        with tokenize.open(file_name) as file:
            return file.read()

    def exists(self, file_name: str) -> bool:
        return os.path.isfile(file_name)
