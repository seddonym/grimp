from __future__ import annotations
from typing import List, Optional, Sequence, Set, Tuple
from grimp.application.ports.graph import DetailedImport
from grimp.domain.analysis import PackageDependency
from grimp.domain.valueobjects import Layer, Module
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]
from grimp.exceptions import ModuleNotPresent, NoSuchContainer
from grimp.adaptors import _layers
from grimp.application.ports import graph


class ImportGraph(graph.ImportGraph):
    """
    Rust-backed implementation of the ImportGraph.
    """

    def __init__(self) -> None:
        super().__init__()
        self._rustgraph = rust.Graph()

    @property
    def modules(self) -> Set[str]:
        return self._rustgraph.get_modules()

    def add_module(self, module: str, is_squashed: bool = False) -> None:
        ancestor_squashed_module = self._find_ancestor_squashed_module(module)
        if ancestor_squashed_module:
            raise ValueError(
                f"Module is a descendant of squashed module {ancestor_squashed_module}."
            )

        if module in self.modules:
            if self.is_module_squashed(module) != is_squashed:
                raise ValueError(
                    "Cannot add a squashed module when it is already present in the graph as "
                    "an unsquashed module, or vice versa."
                )
        self._rustgraph.add_module(module, is_squashed)

    def remove_module(self, module: str) -> None:
        self._rustgraph.remove_module(module)

    def squash_module(self, module: str) -> None:
        if module not in self.modules:
            raise ModuleNotPresent(f'"{module}" not present in the graph.')
        self._rustgraph.squash_module(module)

    def _find_ancestor_squashed_module(self, module: str) -> Optional[str]:
        """
        Return the name of a squashed module that is an ancestor of the supplied module, or None
        if no such module exists.
        """
        try:
            parent = Module(module).parent.name
        except ValueError:
            # The module has no more ancestors.
            return None

        if parent in self.modules and self.is_module_squashed(parent):
            return parent
        else:
            return self._find_ancestor_squashed_module(parent)

    def is_module_squashed(self, module: str) -> bool:
        if module not in self.modules:
            raise ModuleNotPresent(f'"{module}" not present in the graph.')
        return self._rustgraph.is_module_squashed(module)

    def add_import(
        self,
        *,
        importer: str,
        imported: str,
        line_number: Optional[int] = None,
        line_contents: Optional[str] = None,
    ) -> None:
        self._rustgraph.add_import(
            importer=importer,
            imported=imported,
            line_number=line_number,
            line_contents=line_contents,
        )

    def remove_import(self, *, importer: str, imported: str) -> None:
        return self._rustgraph.remove_import(importer=importer, imported=imported)

    def count_imports(self) -> int:
        return self._rustgraph.count_imports()

    def find_children(self, module: str) -> Set[str]:
        # It doesn't make sense to find the children of a squashed module, as we don't store
        # the children in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find children of a squashed module.")
        return self._rustgraph.find_children(module)

    def find_descendants(self, module: str) -> Set[str]:
        # It doesn't make sense to find the descendants of a squashed module, as we don't store
        # the descendants in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find descendants of a squashed module.")
        return self._rustgraph.find_descendants(module)

    def direct_import_exists(
        self, *, importer: str, imported: str, as_packages: bool = False
    ) -> bool:
        return self._rustgraph.direct_import_exists(
            importer=importer, imported=imported, as_packages=as_packages
        )

    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        return self._rustgraph.find_modules_directly_imported_by(module)

    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        if module in self._rustgraph.get_modules():
            # TODO panics if module isn't in modules.
            return self._rustgraph.find_modules_that_directly_import(module)
        return set()

    def get_import_details(self, *, importer: str, imported: str) -> List[DetailedImport]:
        return self._rustgraph.get_import_details(
            importer=importer,
            imported=imported,
        )

    def find_downstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        return self._rustgraph.find_downstream_modules(module, as_package)

    def find_upstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        return self._rustgraph.find_upstream_modules(module, as_package)

    def find_shortest_chain(self, importer: str, imported: str) -> tuple[str, ...] | None:
        for module in (importer, imported):
            if module not in self.modules:
                raise ValueError(f"Module {module} is not present in the graph.")

        chain = self._rustgraph.find_shortest_chain(importer, imported)
        return tuple(chain) if chain else None

    def find_shortest_chains(
        self, importer: str, imported: str, as_packages: bool = True
    ) -> Set[Tuple[str, ...]]:
        return self._rustgraph.find_shortest_chains(importer, imported, as_packages)

    def chain_exists(self, importer: str, imported: str, as_packages: bool = False) -> bool:
        return self._rustgraph.chain_exists(importer, imported, as_packages)

    def find_illegal_dependencies_for_layers(
        self,
        layers: Sequence[Layer | str | set[str]],
        containers: set[str] | None = None,
    ) -> set[PackageDependency]:
        layers = _layers.parse_layers(layers)
        try:
            result = self._rustgraph.find_illegal_dependencies_for_layers(
                layers=tuple(
                    {"layers": layer.module_tails, "independent": layer.independent}
                    for layer in layers
                ),
                containers=set(containers) if containers else set(),
            )
        except rust.NoSuchContainer as e:
            raise NoSuchContainer(str(e))

        return _layers._dependencies_from_tuple(result)

    # Dunder methods
    # --------------

    def __deepcopy__(self, memodict: dict) -> "ImportGraph":
        new_graph = ImportGraph()
        new_graph._rustgraph = self._rustgraph.clone()
        return new_graph
