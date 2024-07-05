from __future__ import annotations

from copy import copy
from typing import Dict, List, Optional, Sequence, Set, Tuple, cast

from grimp.algorithms.shortest_path import bidirectional_shortest_path
from grimp.application.ports import graph
from grimp.domain.analysis import PackageDependency
from grimp.domain.valueobjects import Module, Layer
from grimp.exceptions import ModuleNotPresent

from . import _layers


class ImportGraph(graph.ImportGraph):
    """
    Pure Python implementation of the ImportGraph.
    """

    def __init__(self) -> None:
        # Maps all the modules directly imported by each key.
        self._importeds_by_importer: Dict[str, Set[str]] = {}
        # Maps all the modules that directly import each key.
        self._importers_by_imported: Dict[str, Set[str]] = {}

        self._edge_count = 0

        # Instantiate a dict that stores the details for all direct imports.
        self._import_details: Dict[str, List[graph.DetailedImport]] = {}
        self._squashed_modules: Set[str] = set()

    # Dunder methods
    # --------------

    def __deepcopy__(self, memodict: Dict) -> "ImportGraph":
        new_graph = ImportGraph()
        new_graph._importeds_by_importer = {
            key: value.copy() for key, value in self._importeds_by_importer.items()
        }
        new_graph._importers_by_imported = {
            key: value.copy() for key, value in self._importers_by_imported.items()
        }
        new_graph._edge_count = self._edge_count

        # Note: this copies the dictionaries containing each import detail
        # by *reference*, so be careful about mutating the import details
        # dictionaries internally.
        new_graph._import_details = {
            key: value.copy() for key, value in self._import_details.items()
        }

        new_graph._squashed_modules = self._squashed_modules.copy()

        return new_graph

    # Mechanics
    # ---------

    @property
    def modules(self) -> Set[str]:
        # Note: wrapping this in a set() makes it 10 times slower to build the graph!
        # As a further optimisation, we use the _StringSet type alias instead of looking up Set[str]
        # when casting.
        return cast(_StringSet, self._importeds_by_importer.keys())

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

        self._importeds_by_importer.setdefault(module, set())
        self._importers_by_imported.setdefault(module, set())

        if is_squashed:
            self._mark_module_as_squashed(module)

    def remove_module(self, module: str) -> None:
        if module not in self.modules:
            # TODO: rethink this behaviour.
            return

        for imported in copy(self.find_modules_directly_imported_by(module)):
            self.remove_import(importer=module, imported=imported)
        for importer in copy(self.find_modules_that_directly_import(module)):
            self.remove_import(importer=importer, imported=module)
        del self._importeds_by_importer[module]
        del self._importers_by_imported[module]

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
                    "line_number": cast(int, line_number),
                    "line_contents": cast(str, line_contents),
                }
            )

        importer_map = self._importeds_by_importer.setdefault(importer, set())
        imported_map = self._importers_by_imported.setdefault(imported, set())
        if imported not in importer_map:
            # (Alternatively could check importer in imported_map.)
            importer_map.add(imported)
            imported_map.add(importer)
            self._edge_count += 1

        # Also ensure they have entry in other maps.
        self._importeds_by_importer.setdefault(imported, set())
        self._importers_by_imported.setdefault(importer, set())

    def remove_import(self, *, importer: str, imported: str) -> None:
        if imported in self._importeds_by_importer[importer]:
            self._importeds_by_importer[importer].remove(imported)
            self._importers_by_imported[imported].remove(importer)
            self._edge_count -= 1

            # Clean up import details.
            if importer in self._import_details:
                new_details = [
                    details
                    for details in self._import_details[importer]
                    if details["imported"] != imported
                ]
                if new_details:
                    self._import_details[importer] = new_details
                else:
                    del self._import_details[importer]

    def count_imports(self) -> int:
        return self._edge_count

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
            imported_by_importer = self.find_modules_directly_imported_by(candidate_importer)
            for candidate_imported in imported_modules:
                if candidate_imported in imported_by_importer:
                    return True
        return False

    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        return self._importeds_by_importer[module]

    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        return self._importers_by_imported[module]

    def get_import_details(self, *, importer: str, imported: str) -> List[graph.DetailedImport]:
        import_details_for_importer = self._import_details.get(importer, [])
        # Only include the details for the imported module.
        # Note: we copy each details dictionary at this point, as our deepcopying
        # only copies the dictionaries by reference.
        return [i.copy() for i in import_details_for_importer if i["imported"] == imported]

    # Indirect imports
    # ----------------

    def find_downstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        # TODO optimise for as_package.
        if as_package:
            source_modules = self._all_modules_in_package(module)
        else:
            source_modules = {module}

        downstream_modules = set()

        for candidate in filter(lambda m: m not in source_modules, self.modules):
            for source_module in source_modules:
                if self.chain_exists(importer=candidate, imported=source_module):
                    downstream_modules.add(candidate)
                    break

        return downstream_modules

    def find_upstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        # TODO optimise for as_package.
        if as_package:
            destination_modules = self._all_modules_in_package(module)
        else:
            destination_modules = {module}

        upstream_modules = set()

        for candidate in filter(lambda m: m not in destination_modules, self.modules):
            for destination_module in destination_modules:
                if self.chain_exists(importer=destination_module, imported=candidate):
                    upstream_modules.add(candidate)
                    break

        return upstream_modules

    def find_shortest_chain(self, importer: str, imported: str) -> Optional[Tuple[str, ...]]:
        for module in (importer, imported):
            if module not in self.modules:
                raise ValueError(f"Module {module} is not present in the graph.")

        return self._find_shortest_chain(importer=importer, imported=imported)

    def find_shortest_chains(
        self, importer: str, imported: str, as_packages: bool = True
    ) -> Set[Tuple[str, ...]]:
        """
        Find the shortest import chains that exist between the importer and imported, and
        between any modules contained within them if as_packages is True. Only one chain per
        upstream/downstream pair will be included. Any chains that are contained within other
        chains in the result set will be excluded.

        The default behavior is to treat the import and imported as packages, however, if
        as_packages is False, both the importer and imported will be treated as modules instead.

        Returns:
            A set of tuples of strings. Each tuple is ordered from importer to imported modules.
        """
        shortest_chains = set()

        upstream_modules = (
            {imported} if not as_packages else self._all_modules_in_package(imported)
        )
        downstream_modules = (
            {importer} if not as_packages else self._all_modules_in_package(importer)
        )

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
                shortest_chain = self._find_shortest_chain(imported=upstream, importer=downstream)
                if shortest_chain:
                    shortest_chains.add(shortest_chain)
                self._hide_any_existing_imports(imports_by_downstream_module)
            self._hide_any_existing_imports(imports_of_upstream_module)

        # Reveal all the hidden imports.
        for imports in map_of_imports.values():
            self._reveal_imports(imports)
        self._reveal_imports(imports_between_modules)

        return shortest_chains

    def chain_exists(self, importer: str, imported: str, as_packages: bool = False) -> bool:
        if not as_packages:
            return bool(self._find_shortest_chain(importer=importer, imported=imported))

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

    # High level analysis

    def find_illegal_dependencies_for_layers(
        self,
        layers: Sequence[Layer | str | set[str]],
        containers: set[str] | None = None,
    ) -> set[PackageDependency]:
        layers = _layers.parse_layers(layers)
        return _layers.find_illegal_dependencies(
            graph=self, layers=layers, containers=containers or set()
        )

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

    def _find_all_imports_between_modules(self, modules: Set[str]) -> Set[Tuple[str, str]]:
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
            if self.direct_import_exists(importer=importer, imported=imported):
                # Low-level removal from import graph (but leaving other metadata in place).
                self._importeds_by_importer[importer].remove(imported)
                self._importers_by_imported[imported].remove(importer)

    def _reveal_imports(self, imports: Set[Tuple[str, str]]) -> None:
        """
        Given a set of direct imports that were hidden by _hide_any_existing_imports, add them back.

        Args:
            imports: Set of direct imports, in the form (importer, imported).
        """
        for importer, imported in tuple(imports):
            # Low-level addition to import graph.
            self._importeds_by_importer[importer].add(imported)
            self._importers_by_imported[imported].add(importer)

    def _find_shortest_chain(self, importer: str, imported: str) -> Optional[Tuple[str, ...]]:
        # Similar to find_shortest_chain but without bothering to check if the modules are
        # in the graph first.
        return bidirectional_shortest_path(
            importers_by_imported=self._importers_by_imported,
            importeds_by_importer=self._importeds_by_importer,
            importer=importer,
            imported=imported,
        )


_StringSet = Set[str]
