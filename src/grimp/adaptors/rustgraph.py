from typing import Any, Dict, Iterator, List, Optional, Set, Tuple, Union

import retworkx

from grimp.application.ports import graph
from grimp.domain.valueobjects import Module
from grimp.exceptions import ModuleNotPresent
from grimp.helpers import wrap_generator


class ImportGraph(graph.AbstractImportGraph):
    """
    Implementation of the ImportGraph, backed by a rustworkx directional graph.
    """

    def __init__(self) -> None:
        self._retworkx_graph = retworkx.PyDiGraph(multigraph=False)
        # Instantiate a dict that stores the details for all direct imports.
        self._import_details: Dict[str, List[Dict[str, Any]]] = {}
        self._squashed_modules: Set[str] = set()
        self._node_lookup: Dict[str, int] = {}

    # Mechanics
    # ---------

    @property
    def modules(self) -> Set[str]:
        # Recasting the nodes to a set each time is fairly expensive; this significantly speeds
        # up the building of the graph.
        if not hasattr(self, "_modules"):
            self._modules = set(self._node_lookup.keys())
        return self._modules

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

        node = self._retworkx_graph.add_node(module)
        self._modules.add(module)
        self._node_lookup[module] = node

        if is_squashed:
            self._mark_module_as_squashed(module)

    def remove_module(self, module: str) -> None:
        if module in self.modules:
            node = self._node_lookup.pop(module)
            self._retworkx_graph.remove_node(node)
            self._modules.remove(module)

    def squash_module(self, module: str) -> None:
        if self.is_module_squashed(module):
            return

        squashed_root = module
        descendants = self.find_descendants(squashed_root)

        # Add imports to/from the root.
        for descendant in descendants:
            for imported_module in self.find_modules_directly_imported_by(descendant):
                self.add_import(importer=squashed_root, imported=imported_module)
            for importing_module in self.find_modules_that_directly_import(descendant):
                self.add_import(importer=importing_module, imported=squashed_root)

        # Now we've added imports to/from the root, we can delete the root's descendants.
        for descendant in descendants:
            self.remove_module(descendant)

        self._mark_module_as_squashed(squashed_root)

    def is_module_squashed(self, module: str) -> bool:
        if module not in self.modules:
            raise ModuleNotPresent(f'"{module}" not present in the graph.')

        return module in self._squashed_modules

    def add_import(
        self,
        *,
        importer: str,
        imported: str,
        line_number: Optional[int] = None,
        line_contents: Optional[str] = None,
    ) -> None:
        if any((line_number, line_contents)):
            if not all((line_number, line_contents)):
                raise ValueError(
                    "Line number and contents must be provided together, or not at all."
                )
            self._import_details.setdefault(importer, [])
            self._import_details[importer].append(
                {
                    "importer": importer,
                    "imported": imported,
                    "line_number": line_number,
                    "line_contents": line_contents,
                }
            )
        for module in (importer, imported):
            if module not in self.modules:
                self.add_module(module)
        importer_node = self._node_lookup[importer]
        imported_node = self._node_lookup[imported]
        self._retworkx_graph.add_edge(importer_node, imported_node, None)

    def remove_import(self, *, importer: str, imported: str) -> None:
        importer_node = self._node_lookup[importer]
        imported_node = self._node_lookup[imported]
        self._retworkx_graph.remove_edge(importer_node, imported_node)

    def count_imports(self) -> int:
        return self._retworkx_graph.num_edges()

    # Descendants
    # -----------

    def find_children(self, module: str) -> Set[str]:
        # It doesn't make sense to find the children of a squashed module, as we don't store
        # the children in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find children of a squashed module.")

        children = set()
        for potential_child in self.modules:
            if Module(potential_child).is_child_of(Module(module)):
                children.add(potential_child)
        return children

    def find_descendants(self, module: str) -> Set[str]:
        # It doesn't make sense to find the descendants of a squashed module, as we don't store
        # the descendants in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find descendants of a squashed module.")

        descendants = set()
        for potential_descendant in self.modules:
            if Module(potential_descendant).is_descendant_of(Module(module)):
                descendants.add(potential_descendant)
        return descendants

    # Direct imports
    # --------------

    def direct_import_exists(
        self, *, importer: str, imported: str, as_packages: bool = False
    ) -> bool:
        """
        Whether or not the importer module directly imports the imported module.
        """
        if not as_packages:
            return imported in self.find_modules_directly_imported_by(importer)

        importer_modules = self._all_modules_in_package(importer)
        imported_modules = self._all_modules_in_package(imported)

        if importer_modules & imported_modules:
            # If there are shared modules between the two, one of the modules is a descendant
            # of the other (or they're both the same module). This doesn't make sense in
            # this context, so raise an exception.
            raise ValueError("Modules have shared descendants.")

        # Return True as soon as we find a path between any of the modules in the subpackages.
        for candidate_importer in importer_modules:
            imported_by_importer = self.find_modules_directly_imported_by(
                candidate_importer
            )
            for candidate_imported in imported_modules:
                if candidate_imported in imported_by_importer:
                    return True
        return False

    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        module_node = self._node_lookup[module]
        return set(self._retworkx_graph.successors(module_node))

    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        module_node = self._node_lookup[module]
        return set(self._retworkx_graph.predecessors(module_node))

    def get_import_details(
        self, *, importer: str, imported: str
    ) -> List[Dict[str, Union[str, int]]]:
        import_details_for_importer = self._import_details.get(importer, [])
        # Only include the details for the imported module.
        return [i for i in import_details_for_importer if i["imported"] == imported]

    # Indirect imports
    # ----------------

    def find_downstream_modules(
        self, module: str, as_package: bool = False
    ) -> Set[str]:
        if as_package:
            source_modules = self._all_modules_in_package(module)
        else:
            source_modules = {module}

        ancestor_nodes = set()
        for source_module in source_modules:
            ancestor_nodes |= retworkx.ancestors(
                self._retworkx_graph, self._node_lookup[source_module]
            )

        downstream_modules = {
            self._retworkx_graph[node] for node in ancestor_nodes
        } - source_modules

        return downstream_modules

    def find_upstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        if as_package:
            source_modules = self._all_modules_in_package(module)
        else:
            source_modules = {module}

        descendant_nodes = set()
        for source_module in source_modules:
            descendant_nodes |= retworkx.descendants(
                self._retworkx_graph, self._node_lookup[source_module]
            )

        upstream_modules = {
            self._retworkx_graph[node] for node in descendant_nodes
        } - source_modules

        return upstream_modules

    def find_shortest_chain(
        self, importer: str, imported: str
    ) -> Optional[Tuple[str, ...]]:

        importer_node = self._node_lookup[importer]
        imported_node = self._node_lookup[imported]

        path_dict: Dict[int, List[int]] = retworkx.dijkstra_shortest_paths(
            self._retworkx_graph, source=importer_node, target=imported_node
        )
        try:
            path_nodes = path_dict[imported_node]
        except IndexError:
            return None
        else:
            return tuple(self._retworkx_graph[node] for node in path_nodes)

    def find_shortest_chains(
        self, importer: str, imported: str
    ) -> Set[Tuple[str, ...]]:
        """
        Find the shortest import chains that exist between the importer and imported, and
        between any modules contained within them. Only one chain per upstream/downstream pair
        will be included. Any chains that are contained within other chains in the result set
        will be excluded.

        Returns:
            A set of tuples of strings. Each tuple is ordered from importer to imported modules.
        """
        shortest_chains = set()

        upstream_modules = self._all_modules_in_package(imported)
        downstream_modules = self._all_modules_in_package(importer)

        if upstream_modules & downstream_modules:
            # If there are shared modules between the two, one of the modules is a descendant
            # of the other (or they're both the same module). This doesn't make sense in
            # this context, so raise an exception.
            raise ValueError("Modules have shared descendants.")

        imports_between_modules = self._find_all_imports_between_modules(
            upstream_modules
        ) | self._find_all_imports_between_modules(downstream_modules)
        self._hide_any_existing_imports(imports_between_modules)

        map_of_imports = {}
        for module in upstream_modules | downstream_modules:
            map_of_imports[module] = set(
                (m, module) for m in self.find_modules_that_directly_import(module)
            ) | set((module, m) for m in self.find_modules_directly_imported_by(module))
        for imports in map_of_imports.values():
            self._hide_any_existing_imports(imports)

        for upstream in upstream_modules:
            imports_of_upstream_module = map_of_imports[upstream]
            self._reveal_imports(imports_of_upstream_module)
            for downstream in downstream_modules:
                imports_by_downstream_module = map_of_imports[downstream]
                self._reveal_imports(imports_by_downstream_module)
                shortest_chain = self.find_shortest_chain(
                    imported=upstream, importer=downstream
                )
                if shortest_chain:
                    shortest_chains.add(shortest_chain)
                self._hide_any_existing_imports(imports_by_downstream_module)
            self._hide_any_existing_imports(imports_of_upstream_module)

        # Reveal all the hidden imports.
        for imports in map_of_imports.values():
            self._reveal_imports(imports)
        self._reveal_imports(imports_between_modules)

        return shortest_chains

    def find_all_simple_chains(
        self, importer: str, imported: str
    ) -> Iterator[Tuple[str, ...]]:
        for module in (importer, imported):
            if module not in self.modules:
                raise ModuleNotPresent(f'"{module}" not present in the graph.')

        importer_node = self._node_lookup[importer]
        imported_node = self._node_lookup[imported]
        all_simple_paths = retworkx.all_simple_paths(
            self._retworkx_graph, from_=importer_node, to=imported_node
        )
        # Cast the results to tuples.
        return tuple(
            tuple(self._retworkx_graph[n] for n in path) for path in all_simple_paths
        )

    def chain_exists(
        self, importer: str, imported: str, as_packages: bool = False
    ) -> bool:
        if not as_packages:
            importer_node = self._node_lookup[importer]
            imported_node = self._node_lookup[imported]
            # TODO optimise?
            return bool(
                retworkx.all_simple_paths(
                    self._retworkx_graph, from_=importer_node, to=imported_node
                )
            )

        upstream_modules = self._all_modules_in_package(imported)
        downstream_modules = self._all_modules_in_package(importer)

        if upstream_modules & downstream_modules:
            # If there are shared modules between the two, one of the modules is a descendant
            # of the other (or they're both the same module). This doesn't make sense in
            # this context, so raise an exception.
            raise ValueError("Modules have shared descendants.")

        # Return True as soon as we find a path between any of the modules in the subpackages.
        for upstream in upstream_modules:
            for downstream in downstream_modules:
                if self.chain_exists(imported=upstream, importer=downstream):
                    return True

        return False

    # Private methods

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

    def _mark_module_as_squashed(self, module: str) -> None:
        """
        Set a flag on a module in the graph that it is squashed.
        """
        self._squashed_modules.add(module)

    def _all_modules_in_package(self, module: str) -> Set[str]:
        """
        Return all the modules in the supplied module, including itself.

        If the module is squashed, it will be treated as a single module.
        """
        importer_modules = {module}
        if not self.is_module_squashed(module):
            importer_modules |= self.find_descendants(module)
        return importer_modules

    def _find_all_imports_between_modules(
        self, modules: Set[str]
    ) -> Set[Tuple[str, str]]:
        """
        Return all the imports between the supplied set of modules.

        Return:
            Set of imports, in the form (importer, imported).
        """
        imports = set()
        for importer in modules:
            for imported in self.find_modules_directly_imported_by(importer):
                if imported in modules:
                    imports.add((importer, imported))
        return imports

    def _hide_any_existing_imports(self, imports: Set[Tuple[str, str]]) -> None:
        """
        Temporarily remove the supplied direct imports from the graph.

        If an import is not in the graph, or already hidden, this will have no effect.

        Args:
            imports: Set of direct imports, in the form (importer, imported).
        """
        for importer, imported in tuple(imports):
            importer_node = self._node_lookup[importer]
            imported_node = self._node_lookup[imported]
            if self._retworkx_graph.has_edge(importer_node, imported_node):
                self._retworkx_graph.remove_edge(importer_node, imported_node)

    def _reveal_imports(self, imports: Set[Tuple[str, str]]) -> None:
        """
        Given a set of direct imports that were hidden by _hide_any_existing_imports, add them back.

        Args:
            imports: Set of direct imports, in the form (importer, imported).
        """
        for importer, imported in tuple(imports):
            importer_node = self._node_lookup[importer]
            imported_node = self._node_lookup[imported]
            self._retworkx_graph.add_edge(importer_node, imported_node, None)
