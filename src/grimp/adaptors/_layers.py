from __future__ import annotations

import copy
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Iterator, Sequence, TypedDict, Union

from grimp import Route
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

if TYPE_CHECKING:
    from grimp.adaptors.graph import ImportGraph

from grimp.domain.analysis import PackageDependency
from grimp.exceptions import NoSuchContainer


def find_illegal_dependencies(
    graph: ImportGraph,
    layers: Sequence[Union[str, set[str]]],
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
            levels=_layers_to_levels(layers),
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


def _layers_to_levels(layers: Sequence[Union[str, set[str]]]) -> tuple[set[str], ...]:
    """
    Convert any standalone layers to a one-element level.
    """
    return tuple({layer} if isinstance(layer, str) else set(layer) for layer in layers)


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
if TYPE_CHECKING:
    # TODO: remove TYPE_CHECKING conditional once on Python 3.9.
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


def _search_for_package_dependency(
    higher_layer_package: _Module,
    lower_layer_package: _Module,
    layers: Sequence[str],
    container: str | None,
    graph: ImportGraph,
) -> PackageDependency | None:
    """
    Return a PackageDependency containing illegal chains between two layers, if they exist.
    """
    temp_graph = copy.deepcopy(graph)
    _remove_other_layers(
        temp_graph,
        layers=layers,
        container=container,
        layers_to_preserve=(higher_layer_package, lower_layer_package),
    )
    # Assemble direct imports between the layers, then remove them.
    import_details_between_layers = _pop_direct_imports(
        higher_layer_package=higher_layer_package,
        lower_layer_package=lower_layer_package,
        graph=temp_graph,
    )
    routes: set[Route] = set()

    for import_details_list in import_details_between_layers:
        any_element = tuple(import_details_list)[0]
        routes.add(
            Route(
                heads=frozenset({any_element.importer}),
                middle=(),
                tails=frozenset({any_element.imported}),
            )
        )

    indirect_routes = _get_indirect_routes(
        temp_graph,
        importer_package=lower_layer_package,
        imported_package=higher_layer_package,
    )

    routes |= indirect_routes

    if routes:
        return PackageDependency(
            importer=lower_layer_package.name,
            imported=higher_layer_package.name,
            routes=frozenset(routes),
        )
    else:
        return None


def _get_indirect_routes(
    graph: ImportGraph, importer_package: _Module, imported_package: _Module
) -> set[Route]:
    """
    Squashes the two packages.
    Gets a list of paths between them, called segments.
    Add the heads and tails to the segments.
    """
    temp_graph = copy.deepcopy(graph)

    temp_graph.squash_module(importer_package.name)
    temp_graph.squash_module(imported_package.name)

    middles = _find_middles(
        temp_graph,
        importer=importer_package,
        imported=imported_package,
    )
    return _middles_to_routes(graph, middles, importer=importer_package, imported=imported_package)


def _remove_other_layers(
    graph: ImportGraph,
    layers: Sequence[str],
    container: str | None,
    layers_to_preserve: tuple[_Module, ...],
) -> None:
    for index, layer in enumerate(layers):  # type: ignore
        candidate_layer = _module_from_layer(layer, container)
        if candidate_layer.name in graph.modules and candidate_layer not in layers_to_preserve:
            _remove_layer(graph, layer_package=candidate_layer)


def _remove_layer(graph: ImportGraph, layer_package: _Module) -> None:
    for module in graph.find_descendants(layer_package.name):
        graph.remove_module(module)
    graph.remove_module(layer_package.name)


def _pop_direct_imports(
    higher_layer_package, lower_layer_package, graph: ImportGraph
) -> set[frozenset[_Link]]:
    import_details_set: set[frozenset[_Link]] = set()

    lower_layer_modules = {lower_layer_package.name} | graph.find_descendants(
        lower_layer_package.name
    )
    for lower_layer_module in lower_layer_modules:
        imported_modules = graph.find_modules_directly_imported_by(lower_layer_module).copy()
        for imported_module in imported_modules:
            if _Module(imported_module) == higher_layer_package or _Module(
                imported_module
            ).is_descendant_of(higher_layer_package):
                import_details = frozenset(
                    {
                        _Link(
                            importer=lower_layer_module,
                            imported=imported_module,
                        ),
                    }
                )
                import_details_set.add(import_details)
                graph.remove_import(importer=lower_layer_module, imported=imported_module)
    return import_details_set


def _find_middles(graph: ImportGraph, importer: _Module, imported: _Module) -> set[_Chain]:
    """
    Return set of headless and tailless chains.
    """
    middles: set[_Chain] = set()

    for chain in _pop_shortest_chains(graph, importer=importer.name, imported=imported.name):
        if len(chain) == 2:
            raise ValueError("Direct chain found - these should have been removed.")
        middles.add(chain[1:-1])

    return middles


def _middles_to_routes(
    graph: ImportGraph, middles: set[_Chain], importer: _Module, imported: _Module
) -> set[Route]:
    """
    Build a set of routes from the chains between one package and another.

    The middles are the chains that exist from the importer package to
    the importer package. This function works out the head and tail packages of
    those chains by consulting the graph.
    """
    routes: set[Route] = set()

    for middle in middles:
        heads: set[str] = set()
        imported_module = middle[0]
        candidate_modules = sorted(graph.find_modules_that_directly_import(imported_module))
        for module in [
            m
            for m in candidate_modules
            if _Module(m) == importer or _Module(m).is_descendant_of(importer)
        ]:
            heads.add(module)

        tails: set[str] = set()
        importer_module = middle[-1]
        candidate_modules = sorted(graph.find_modules_directly_imported_by(importer_module))
        for module in [
            m
            for m in candidate_modules
            if _Module(m) == imported or _Module(m).is_descendant_of(imported)
        ]:
            tails.add(module)

        routes.add(
            Route(
                heads=frozenset(heads),
                middle=middle,
                tails=frozenset(tails),
            )
        )

    return routes


def _pop_shortest_chains(
    graph: ImportGraph, importer: str, imported: str
) -> Iterator[tuple[str, ...]]:
    chain: tuple[str, ...] | bool | None = True
    while chain:
        chain = graph.find_shortest_chain(importer, imported)
        if chain:
            # Remove chain of imports from graph.
            for index in range(len(chain) - 1):
                graph.remove_import(importer=chain[index], imported=chain[index + 1])
            yield chain
