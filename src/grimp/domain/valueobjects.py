from dataclasses import dataclass, astuple
from typing import Set, Any


@dataclass(frozen=True, repr=False, eq=False)
class ValueObject:
    def __repr__(self) -> str:
        return "<{}: {}>".format(self.__class__.__name__, self)

    # We use a custom __eq__ method to enforce the liskov principle.
    # e.g. SpecialModule("foo") == Module("foo") where SpecialModule is a subclass of Module.
    def __eq__(self, other: Any) -> bool:
        if isinstance(other, type(self)) or isinstance(self, type(other)):
            return hash(self) == hash(other)
        else:
            return False

    def __hash__(self) -> int:
        return hash(astuple(self))


@dataclass(frozen=True, repr=False, eq=False)
class Module(ValueObject):
    """
    A Python module.
    """

    # The fully qualified name of a Python module, e.g. 'package.foo.bar'.
    name: str

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


@dataclass(frozen=True, repr=False, eq=False)
class DirectImport(ValueObject):
    """
    An import between one module and another.
    """

    importer: Module
    imported: Module
    line_number: int
    line_contents: str

    def __str__(self) -> str:
        return f"{self.importer} -> {self.imported} (l. {self.line_number})"


@dataclass(frozen=True, repr=False, eq=False)
class Layer(ValueObject):
    """
    A layer within a layered architecture.

    If layer.independent is True then the modules within the layer are considered
    independent. This is the default.
    """

    module_tails: Set[str]
    independent: bool

    # A custom `__init__` is needed since `module_tails` is a variadic argument.
    def __init__(self, *module_tails: str, independent: bool = True) -> None:
        # `object.__setattr__` is needed since the dataclass is frozen.
        object.__setattr__(self, "module_tails", set(module_tails))
        object.__setattr__(self, "independent", independent)

    def __str__(self) -> str:
        return f"{self.module_tails}, independent={self.independent}"
