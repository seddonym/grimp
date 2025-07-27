from dataclasses import dataclass
from typing import Set


@dataclass(frozen=True)
class Module:
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


@dataclass(frozen=True)
class DirectImport:
    """
    An import between one module and another.
    """

    importer: Module
    imported: Module
    line_number: int
    line_contents: str

    def __str__(self) -> str:
        return f"{self.importer} -> {self.imported} (l. {self.line_number})"


@dataclass(frozen=True)
class Layer:
    """
    A layer within a layered architecture.

    If layer.independent is True then the modules within the layer are considered
    independent. This is the default.
    """

    module_tails: Set[str]
    independent: bool
    closed: bool

    # A custom `__init__` is needed since `module_tails` is a variadic argument.
    def __init__(self, *module_tails: str, independent: bool = True, closed: bool = False) -> None:
        # `object.__setattr__` is needed since the dataclass is frozen.
        object.__setattr__(self, "module_tails", set(module_tails))
        object.__setattr__(self, "independent", independent)
        object.__setattr__(self, "closed", closed)

    def __str__(self) -> str:
        module_tails = sorted(self.module_tails)
        return f"{module_tails}, independent={self.independent}, closed={self.closed}"
