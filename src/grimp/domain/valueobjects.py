from typing import Any


class ValueObject:
    def __repr__(self) -> str:
        return "<{}: {}>".format(self.__class__.__name__, self)

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, type(self)) or isinstance(self, type(other)):
            return hash(self) == hash(other)
        else:
            return False

    def __hash__(self) -> int:
        return hash(str(self))


class Module(ValueObject):
    """
    A Python module.
    """

    __slots__ = ("name",)

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
        return self.name.split(".")[0]

    @property
    def root(self) -> "Module":
        """
        The root package.
        """
        return Module(self.package_name)

    @property
    def parent(self) -> "Module":
        components = self.name.split(".")
        if len(components) == 1:
            raise ValueError("Module has no parent.")
        return Module(".".join(components[:-1]))

    def is_child_of(self, module: "Module") -> bool:
        try:
            return module == self.parent
        except ValueError:
            # If this module has no parent, then it cannot be a child of the supplied module.
            return False

    def is_descendant_of(self, module: "Module") -> bool:
        return self.name.startswith(f"{module.name}.")


class DirectImport(ValueObject):
    """
    An import between one module and another.
    """

    def __init__(
        self,
        *,
        importer: Module,
        imported: Module,
        line_number: int,
        line_contents: str,
    ) -> None:
        self.importer = importer
        self.imported = imported
        self.line_number = line_number
        self.line_contents = line_contents

    def __str__(self) -> str:
        return "{} -> {} (l. {})".format(self.importer, self.imported, self.line_number)

    def __hash__(self) -> int:
        return hash((str(self), self.line_contents))


class Layer(ValueObject):
    """
    A layer within a layered architecture.

    If layer.independent is True then the modules within the layer are considered
    independent. This is the default.
    """

    def __init__(
        self,
        *module_tails: str,
        independent: bool = True,
    ) -> None:
        self.module_tails = set(module_tails)
        self.independent = independent

    def __str__(self) -> str:
        return f"{self.module_tails}, independent={self.independent}"
