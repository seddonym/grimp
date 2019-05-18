from typing import Set, Tuple, Optional, Dict, Union, List, Any

import networkx  # type: ignore
import networkx.algorithms  # type: ignore

from grimp.application.ports import graph
from grimp.domain.valueobjects import Module


class ImportGraph(graph.AbstractImportGraph):
    """
    Implementation of the ImportGraph, backed by a networkx directional graph.
    """
    def __init__(self) -> None:
        self._networkx_graph = networkx.DiGraph()
        # Instantiate a dict that stores the details for all direct imports.
        self._import_details: Dict[str, List[Dict[str, Any]]] = {}
        self._squashed_modules: Set[str] = set()

    # Mechanics
    # ---------

    @property
    def modules(self) -> Set[str]:
        return set(self._networkx_graph.nodes)

    def add_module(self, module: str, is_squashed: bool = False) -> None:
        ancestor_squashed_module = self._find_ancestor_squashed_module(module)
        if ancestor_squashed_module:
            raise ValueError(
                f'Module is a descendant of squashed module {ancestor_squashed_module}.')

        if module in self.modules:
            if self._is_existing_module_squashed(module) != is_squashed:
                raise ValueError(
                    'Cannot add a squashed module when it is already present in the graph as '
                    'an unsquashed module, or vice versa.')

        self._networkx_graph.add_node(module)

        if is_squashed:
            self._mark_module_as_squashed(module)

    def remove_module(self, module: str) -> None:
        if module in self.modules:
            self._networkx_graph.remove_node(module)

    def add_import(
        self, *,
        importer: str,
        imported: str,
        line_number: Optional[int] = None,
        line_contents: Optional[str] = None
    ) -> None:
        if any((line_number, line_contents)):
            if not all((line_number, line_contents)):
                raise ValueError(
                    'Line number and contents must be provided together, or not at all.')
            self._import_details.setdefault(importer, [])
            self._import_details[importer].append({
                'importer': importer,
                'imported': imported,
                'line_number': line_number,
                'line_contents': line_contents,
            })

        self._networkx_graph.add_edge(importer, imported)

    def remove_import(self, *, importer: str, imported: str) -> None:
        self._networkx_graph.remove_edge(importer, imported)

    def count_imports(self) -> int:
        return len(self._networkx_graph.edges)

    # Descendants
    # -----------

    def find_children(self, module: str) -> Set[str]:
        # It doesn't make sense to find the children of a squashed module, as we don't store
        # the children in the graph.
        if self._is_existing_module_squashed(module):
            raise ValueError('Cannot find children of a squashed module.')

        children = set()
        for potential_child in self.modules:
            if Module(potential_child).is_child_of(Module(module)):
                children.add(potential_child)
        return children

    def find_descendants(self, module: str) -> Set[str]:
        # It doesn't make sense to find the descendants of a squashed module, as we don't store
        # the descendants in the graph.
        if self._is_existing_module_squashed(module):
            raise ValueError('Cannot find descendants of a squashed module.')

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

        importer_modules = {importer}
        if not self._is_existing_module_squashed(importer):
            importer_modules |= self.find_descendants(importer)

        imported_modules = {imported}
        if not self._is_existing_module_squashed(imported):
            imported_modules |= self.find_descendants(imported)

        if importer_modules & imported_modules:
            # If there are shared modules between the two, one of the modules is a descendant
            # of the other (or they're both the same module). This doesn't make sense in
            # this context, so raise an exception.
            raise ValueError('Modules have shared descendants.')

        # Return True as soon as we find a path between any of the modules in the subpackages.
        for candidate_importer in importer_modules:
            imported_by_importer = self.find_modules_directly_imported_by(candidate_importer)
            for candidate_imported in imported_modules:
                if candidate_imported in imported_by_importer:
                    return True
        return False

    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        return set(self._networkx_graph.successors(module))

    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        return set(self._networkx_graph.predecessors(module))

    def get_import_details(
        self,
        *,
        importer: str,
        imported: str
    ) -> List[Dict[str, Union[str, int]]]:
        import_details_for_importer = self._import_details.get(importer, [])
        # Only include the details for the imported module.
        return [i for i in import_details_for_importer if i['imported'] == imported]

    # Indirect imports
    # ----------------

    def find_downstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        # TODO optimise for as_package.
        source_modules = {module}
        if as_package:
            # For squashed modules, the behaviour is the same regardless of whether or not
            # we specify as_package, as the node itself represents the package.
            if not self._is_existing_module_squashed(module):
                source_modules.update(self.find_descendants(module))

        downstream_modules = set()

        for candidate in filter(lambda m: m not in source_modules, self.modules):
            for source_module in source_modules:
                if networkx.algorithms.has_path(self._networkx_graph, candidate, source_module):
                    downstream_modules.add(candidate)
                    break

        return downstream_modules

    def find_upstream_modules(
        self, module: str, as_package: bool = False
    ) -> Set[str]:
        # TODO optimise for as_package.
        destination_modules = {module}
        if as_package:
            # For squashed modules, the behaviour is the same regardless of whether or not
            # we specify as_package, as the node itself represents the package.
            if not self._is_existing_module_squashed(module):
                destination_modules.update(self.find_descendants(module))

        upstream_modules = set()

        for candidate in filter(lambda m: m not in destination_modules, self.modules):
            for destination_module in destination_modules:
                if networkx.algorithms.has_path(
                    self._networkx_graph,
                    destination_module,
                    candidate
                ):
                    upstream_modules.add(candidate)
                    break

        return upstream_modules

    def find_shortest_chain(
        self, importer: str, imported: str,
    ) -> Optional[Tuple[str, ...]]:
        try:
            return tuple(networkx.algorithms.shortest_path(self._networkx_graph,
                                                           source=importer,
                                                           target=imported))
        except networkx.NetworkXNoPath:
            return None

    def chain_exists(
        self, importer: str, imported: str, as_packages=False,
    ) -> bool:
        if not as_packages:
            return networkx.algorithms.has_path(self._networkx_graph,
                                                source=importer,
                                                target=imported)
        upstream_modules = {imported}
        if not self._is_existing_module_squashed(imported):
            upstream_modules |= self.find_descendants(imported)

        downstream_modules = {importer}
        if not self._is_existing_module_squashed(importer):
            downstream_modules |= self.find_descendants(importer)

        if upstream_modules & downstream_modules:
            # If there are shared modules between the two, one of the modules is a descendant
            # of the other (or they're both the same module). This doesn't make sense in
            # this context, so raise an exception.
            raise ValueError('Modules have shared descendants.')

        # Return True as soon as we find a path between any of the modules in the subpackages.
        for upstream in upstream_modules:
            for downstream in downstream_modules:
                if self.chain_exists(imported=upstream,
                                     importer=downstream):
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
            # The module no more ancestors.
            return None

        if self._is_existing_module_squashed(parent):
            return parent
        else:
            return self._find_ancestor_squashed_module(parent)

    def _is_existing_module_squashed(self, module: str) -> bool:
        """
        Return whether a module that currently exists in the graph is squashed.
        """
        return module in self._squashed_modules

    def _mark_module_as_squashed(self, module: str) -> None:
        """
        Set a flag on a module in the graph that it is squashed.
        """
        self._squashed_modules.add(module)
