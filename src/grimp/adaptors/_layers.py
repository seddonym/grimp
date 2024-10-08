from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Iterator, Sequence, TypedDict

from grimp import Route
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

if TYPE_CHECKING:
    from grimp.adaptors.graph import ImportGraph

from grimp.domain.analysis import PackageDependency
from grimp.exceptions import NoSuchContainer
from grimp.domain.valueobjects import Layer


def parse_layers(layers: Sequence[Layer | str | set[str]]) -> tuple[Layer, ...]:
    """
    Convert the passed raw `layers` into `Layer`s.
    """
    out_layers = []
    for layer in layers:
        if isinstance(layer, Layer):
            out_layers.append(layer)
        elif isinstance(layer, str):
            out_layers.append(Layer(layer, independent=True))
        else:
            out_layers.append(Layer(*tuple(layer), independent=True))
    return tuple(out_layers)


def find_illegal_dependencies(
    graph: ImportGraph,
    layers: Sequence[Layer],
    containers: set[str],
) -> set[PackageDependency]:
    """
    Find dependencies that don't conform to the supplied layered architecture.

    See ImportGraph.find_illegal_dependencies_for_layers.

    The only difference between this and the method is that the containers passed in
    is already a (potentially empty) set.
    """
    try:
        rust_package_dependency_tuple = rust.find_illegal_dependencies(
            levels=tuple(
                {"layers": layer.module_tails, "independent": layer.independent}
                for layer in layers
            ),
            containers=set(containers),
            importeds_by_importer=graph._importeds_by_importer,
        )
    except rust.NoSuchContainer as e:
        raise NoSuchContainer(str(e))

    rust_package_dependencies = _dependencies_from_tuple(rust_package_dependency_tuple)
    return rust_package_dependencies


class _RustRoute(TypedDict):
    heads: frozenset[str]
    middle: tuple[str, ...]
    tails: frozenset[str]


class _RustPackageDependency(TypedDict):
    importer: str
    imported: str
    routes: tuple[_RustRoute, ...]


def _dependencies_from_tuple(
    rust_package_dependency_tuple: tuple[_RustPackageDependency, ...]
) -> set[PackageDependency]:
    return {
        PackageDependency(
            imported=dep_dict["imported"],
            importer=dep_dict["importer"],
            routes=frozenset(
                {
                    Route(
                        heads=route_dict["heads"],
                        middle=route_dict["middle"],
                        tails=route_dict["tails"],
                    )
                    for route_dict in dep_dict["routes"]
                }
            ),
        )
        for dep_dict in rust_package_dependency_tuple
    }


class _Module:
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

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, self.__class__):
            return hash(self) == hash(other)
        else:
            return False

    def __hash__(self) -> int:
        return hash(str(self))

    def is_descendant_of(self, module: "_Module") -> bool:
        return self.name.startswith(f"{module.name}.")


@dataclass(frozen=True)
class _Link:
    importer: str
    imported: str


# A chain of modules, each of which imports the next.
_Chain = tuple[str, ...]


def _generate_module_permutations(
    graph: ImportGraph,
    layers: Sequence[str],
    containers: set[str],
) -> Iterator[tuple[_Module, _Module, str | None]]:
    """
    Return all possible combinations of higher level and lower level modules, in pairs.

    Each pair of modules consists of immediate children of two different layers. The first
    module is in a layer higher than the layer of the second module. This means the first
    module is allowed to import the second, but not the other way around.

    Returns:
        module_in_higher_layer, module_in_lower_layer, container
    """
    # If there are no containers, we still want to run the loop once.
    quasi_containers = containers or [None]

    for container in quasi_containers:
        for index, higher_layer in enumerate(layers):
            higher_layer_module = _module_from_layer(higher_layer, container)

            if higher_layer_module.name not in graph.modules:
                continue

            for lower_layer in layers[index + 1 :]:
                lower_layer_module = _module_from_layer(lower_layer, container)

                if lower_layer_module.name not in graph.modules:
                    continue

                yield higher_layer_module, lower_layer_module, container


def _module_from_layer(layer: str, container: str | None = None) -> _Module:
    if container:
        name = ".".join([container, layer])
    else:
        name = layer
    return _Module(name)
