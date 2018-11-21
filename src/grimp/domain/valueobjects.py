from typing import List, Any


class ValueObject:
    def __repr__(self) -> str:
        return "<{}: {}>".format(self.__class__.__name__, self)

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, self.__class__):
            return str(self) == str(other)
        else:
            return False

    def __hash__(self) -> int:
        return hash(str(self))


class Module(ValueObject):
    """
    A Python module.
    """
    def __init__(self, name: str) -> None:
        """
        Args:
            name: The fully qualified name of a Python module, e.g. 'package.foo.bar'.
        """
        self.name = name

    def __str__(self) -> str:
        return self.name

    @property
    def package_name(self) -> str:
        return self.name.split('.')[0]

    def is_child_of(self, module: 'Module') -> bool:
        return self.name.split('.')[:-1] == module.name.split('.')

    def is_descendant_of(self, module: 'Module') -> bool:
        return self.name.startswith(f'{module.name}.')


class DirectImport(ValueObject):
    """
    An import between one module and another.
    """
    def __init__(self, importer: Module, imported: Module) -> None:
        self.importer = importer
        self.imported = imported

    def __str__(self) -> str:
        return "{} -> {}".format(self.importer, self.imported)


class ImportPath(ValueObject):
    """
    A flow of imports between two modules, from upstream to downstream.
    """
    def __init__(self, *modules: List[Module]) -> None:
        self.modules = modules

    def __str__(self) -> str:
        return ' -> '.join(
            reversed([str(m) for m in self.modules])
        )
