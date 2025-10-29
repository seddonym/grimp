from __future__ import annotations
from typing import List, Optional, Sequence, Set, Tuple, TypedDict
from grimp.domain.analysis import PackageDependency, Route
from grimp.domain.valueobjects import Layer
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]
from grimp.exceptions import (
    ModuleNotPresent,
    NoSuchContainer,
    InvalidModuleExpression,
    InvalidImportExpression,
)


class Import(TypedDict):
    importer: str
    imported: str


# Corresponds to importer, imported.
# Prefer this form to Import, as it's both more lightweight, and hashable.
ImportTuple = Tuple[str, str]


class DetailedImport(Import):
    line_number: int
    line_contents: str


class ImportGraph:
    """
    A Directed Graph of imports between Python modules.
    """

    def __init__(self) -> None:
        super().__init__()
        self._cached_modules: Set[str] | None = None
        self._rustgraph = rust.Graph()

    # Mechanics
    # ---------

    @property
    def modules(self) -> Set[str]:
        """
        The names of all the modules in the graph.
        """
        if self._cached_modules is None:
            self._cached_modules = self._rustgraph.get_modules()
        return self._cached_modules

    def find_matching_modules(self, expression: str) -> Set[str]:
        """
        Find all modules matching the passed expression.

        Args:
            expression: A module expression used for matching.
        Returns:
            A set of module names matching the expression.
        Raises:
            InvalidModuleExpression if the passed expression is invalid.

        Module Expressions
        ==================

        A module expression is used to refer to sets of modules.

        - ``*`` stands in for a module name, without including subpackages.
        - ``**`` includes subpackages too.

        Examples
        --------

        - ``mypackage.foo``:  matches ``mypackage.foo`` exactly.
        - ``mypackage.*``:  matches ``mypackage.foo`` but not ``mypackage.foo.bar``.
        - ``mypackage.*.baz``: matches ``mypackage.foo.baz`` but not ``mypackage.foo.bar.baz``.
        - ``mypackage.*.*``: matches ``mypackage.foo.bar`` and ``mypackage.foobar.baz``.
        - ``mypackage.**``: matches ``mypackage.foo.bar`` and ``mypackage.foo.bar.baz``.
        - ``mypackage.**.qux``: matches ``mypackage.foo.bar.qux`` and ``mypackage.foo.bar.baz.qux``.
        - ``mypackage.foo*``: is not a valid expression. (The wildcard must replace a whole module
          name.)
        """
        try:
            return self._rustgraph.find_matching_modules(expression)
        except rust.InvalidModuleExpression as e:
            raise InvalidModuleExpression(str(e)) from e

    def add_module(self, module: str, is_squashed: bool = False) -> None:
        """
        Add a module to the graph.

        If is_squashed is True, the module should be treated as a 'squashed module'. This means
        the module has a node in the graph that represents both itself and all its descendants.
        Using squashed modules allows you to simplify some parts of the graph, for example if you
        want to include an external package in the graph but don't care about all the dependencies
        within that package.
        """
        self._cached_modules = None
        self._rustgraph.add_module(module, is_squashed)

    def remove_module(self, module: str) -> None:
        """
        Remove a module from the graph, if it exists.

        If the module is not present in the graph, no exception will be raised.
        """
        self._cached_modules = None
        self._rustgraph.remove_module(module)

    def squash_module(self, module: str) -> None:
        """
        'Squash' a module in the graph.

        If the module is not present in the graph, grimp.exceptions.ModuleNotPresent will be raised.

        A squashed module represents both itself and all its descendants. This allow parts of the
        graph to be simplified.
        """
        self._cached_modules = None
        if not self._rustgraph.contains_module(module):
            raise ModuleNotPresent(f'"{module}" not present in the graph.')
        self._rustgraph.squash_module(module)

    def is_module_squashed(self, module: str) -> bool:
        """
        Return whether a module is squashed.

        If the module is not present in the graph, grimp.exceptions.ModuleNotPresent will be raised.
        """
        if not self._rustgraph.contains_module(module):
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
        """
        Add a direct import between two modules to the graph. If the modules are not already
        present, they will be added to the graph.
        """
        self._cached_modules = None
        self._rustgraph.add_import(
            importer=importer,
            imported=imported,
            line_number=line_number,
            line_contents=line_contents,
        )

    def remove_import(self, *, importer: str, imported: str) -> None:
        """
        Remove a direct import between two modules. Does not remove the modules themselves.
        """
        self._cached_modules = None
        return self._rustgraph.remove_import(importer=importer, imported=imported)

    def count_imports(self) -> int:
        """
        Return the number of imports in the graph.
        """
        return self._rustgraph.count_imports()

    # Descendants
    # -----------

    def find_children(self, module: str) -> Set[str]:
        """
        Find all modules one level below the module. For example, the children of
        foo.bar might be foo.bar.one and foo.bar.two, but not foo.bar.two.green.

        Raises:
            ValueError if attempted on a squashed module.
        """
        # It doesn't make sense to find the children of a squashed module, as we don't store
        # the children in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find children of a squashed module.")
        return self._rustgraph.find_children(module)

    def find_descendants(self, module: str) -> Set[str]:
        """
        Find all modules below the module. For example, the descendants of
        foo.bar might be foo.bar.one and foo.bar.two and foo.bar.two.green.

        Raises:
            ValueError if attempted on a squashed module.
        """
        # It doesn't make sense to find the descendants of a squashed module, as we don't store
        # the descendants in the graph.
        if self.is_module_squashed(module):
            raise ValueError("Cannot find descendants of a squashed module.")
        return self._rustgraph.find_descendants(module)

    # Direct imports
    # --------------

    def direct_import_exists(
        self, *, importer: str, imported: str, as_packages: bool = False
    ) -> bool:
        """
        Whether or not the importer module directly imports the imported module.

        Args:
            importer:     Name of the importer module.
            imported:     Name of the imported module.
            as_packages:  Whether or not to treat the supplied modules as individual modules,
                          or as entire packages (including any descendants).
        """
        return self._rustgraph.direct_import_exists(
            importer=importer, imported=imported, as_packages=as_packages
        )

    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        return self._rustgraph.find_modules_directly_imported_by(module)

    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        if self._rustgraph.contains_module(module):
            # TODO panics if module isn't in modules.
            return self._rustgraph.find_modules_that_directly_import(module)
        return set()

    def get_import_details(self, *, importer: str, imported: str) -> List[DetailedImport]:
        """
        Return available metadata relating to the direct imports between two modules, in the form:
        [
            {
                'importer': 'mypackage.importer',
                'imported': 'mypackage.imported',
                'line_number': 5,
                'line_contents': 'from mypackage import imported',
            },
            (additional imports here)
        ]

        If no import exists, or if there are no available details, returns an empty list.

        Note, it is possible for an import to exist, but for there to be no available details.
        For example, if an import has been added by the `add_import` method without line_number and
        line_contents specified.
        """
        return self._rustgraph.get_import_details(
            importer=importer,
            imported=imported,
        )

    def find_matching_direct_imports(self, import_expression: str) -> List[Import]:
        """
        Find all direct imports matching the passed expressions.

        The imports are returned are in the following form:

        [
            {
                'importer': 'mypackage.importer',
                'imported': 'mypackage.imported',
            },
            ...
        ]

        Args:
            import_expression: An expression used for matching importing modules, in the form
                "importer_expression -> imported_expression", where both expressions are
                module expressions.
        Returns:
            A list of direct imports matching the expressions, ordered alphabetically by importer,
            then imported.
            (We return a list rather than a set purely because dictionaries aren't hashable.)
        Raises:
            InvalidImportExpression if either of the passed expressions are invalid.

        See `ImportGraph.find_matching_modules` for a description of module expressions.
        """
        try:
            importer_expression, imported_expression = import_expression.split(" -> ")
        except ValueError:
            raise InvalidImportExpression(f"{import_expression} is not a valid import expression.")

        try:
            return self._rustgraph.find_matching_direct_imports(
                importer_expression=importer_expression, imported_expression=imported_expression
            )
        except rust.InvalidModuleExpression as e:
            raise InvalidImportExpression(
                f"{import_expression} is not a valid import expression."
            ) from e

    # Indirect imports
    # ----------------

    def find_downstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        """
        Return a set of the names of all the modules that import (even indirectly) the
        supplied module name.
        Args:
            module:        The absolute name of the upstream Module.
            as_package: Whether or not to treat the supplied module as an individual module,
                           or as an entire subpackage (including any descendants). If
                           treating it as a subpackage, the result will include downstream
                           modules *external* to the subpackage, and won't include modules within
                           the subpackage.
        Usage:

            # Returns the modules downstream of mypackage.foo.
            import_graph.find_downstream_modules('mypackage.foo')

            # Returns the modules downstream of mypackage.foo, mypackage.foo.one and
            # mypackage.foo.two.
            import_graph.find_downstream_modules('mypackage.foo', as_package=True)
        """
        return self._rustgraph.find_downstream_modules(module, as_package)

    def find_upstream_modules(self, module: str, as_package: bool = False) -> Set[str]:
        """
        Return a set of the names of all the modules that are imported (even indirectly) by the
        supplied module.

        Args:
            module:        The name of the downstream module.
            as_package:    Whether or not to treat the supplied module as an individual module,
                           or as a package (i.e. including any descendants, if there ary any). If
                           treating it as a subpackage, the result will include upstream
                           modules *external* to the subpackage, and won't include modules within
                           the subpackage.
        """
        return self._rustgraph.find_upstream_modules(module, as_package)

    def find_shortest_chain(
        self, importer: str, imported: str, as_packages: bool = False
    ) -> tuple[str, ...] | None:
        """
        Attempt to find the shortest chain of imports between two modules, in the direction
        of importer to imported.

        Optional args:
            as_packages: Whether to treat the supplied modules as individual modules,
                         or as packages (including any descendants, if there are any). If
                         treating them as subpackages, all descendants of the supplied modules
                         will be checked too.

        Returns:
            Tuple of module names, from importer to imported, or None if no chain exists.
        """
        for module in (importer, imported):
            if not self._rustgraph.contains_module(module):
                raise ValueError(f"Module {module} is not present in the graph.")

        chain = self._rustgraph.find_shortest_chain(importer, imported, as_packages)
        return tuple(chain) if chain else None

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
        return self._rustgraph.find_shortest_chains(importer, imported, as_packages)

    def chain_exists(self, importer: str, imported: str, as_packages: bool = False) -> bool:
        """
        Return whether any chain of imports exists between the two modules, in the direction
        of importer to imported. In other words, does the importer depend on the imported?

        Optional args:
            as_packages: Whether to treat the supplied modules as individual modules,
                         or as packages (including any descendants, if there are any). If
                         treating them as subpackages, all descendants of the supplied modules
                         will be checked too.
        """
        return self._rustgraph.chain_exists(importer, imported, as_packages)

    # High level analysis
    # -------------------

    def find_illegal_dependencies_for_layers(
        self,
        layers: Sequence[Layer | str | set[str]],
        containers: set[str] | None = None,
    ) -> set[PackageDependency]:
        """
        Find dependencies that don't conform to the supplied layered architecture.

        'Layers' is an architectural pattern in which a list of modules/packages
        have a dependency direction from high to low. In other words, a higher layer would
        be allowed to import a lower layer, but not the other way around.

        Additionally, multiple modules can be grouped together at the same layer;
        for example `mypackage.utils` and `mypackage.logging` might sit at the bottom, so they
        cannot import from any other layers. To specify that multiple modules should
        be treated as siblings within a single layer, pass a `Layer`. The `Layer.independent`
        field can be used to specify whether the sibling modules should be treated as independent
        - should imports between sibling modules be forbidden (default) or allowed? For backwards
        compatibility it is also possible to pass a simple `set[str]` to describe a layer. In this
        case the sibling modules within the layer will be considered independent.

        By default layers are open. `Layer.closed` can be set to True to create a closed layer.
        Imports from higher to lower layers cannot bypass closed layers - the closed layer must be
        included in the import chain. For example, given the layers high -> mid (closed) -> low then
        all import chains from high -> low must go via mid.

        Arguments:

        - layers:     A sequence, each element of which consists either of a `Layer`, the name
                      of a layer module or a set of sibling modules. If containers
                      are also specified, then these names must be relative to the container.
                      The order is from higher to lower level layers. Any layers that don't
                      exist in the graph will be ignored.
        - containers: The parent modules of the layers, as absolute names that you could import,
                      such as "mypackage.foo". (Optional.)

        Returns the illegal dependencies in the form of a set of PackageDependency objects.
        Each package dependency is for a different permutation of two layers for which there
        is a violation, and contains information about the illegal chains of imports from the
        lower layer (the 'upstream') to the higher layer (the 'downstream').

        Raises NoSuchContainer if the container is not a module in the graph.
        """
        layers = _parse_layers(layers)
        try:
            result = self._rustgraph.find_illegal_dependencies_for_layers(
                layers=tuple(
                    {
                        "layers": layer.module_tails,
                        "independent": layer.independent,
                        "closed": layer.closed,
                    }
                    for layer in layers
                ),
                containers=set(containers) if containers else set(),
            )
        except rust.NoSuchContainer as e:
            raise NoSuchContainer(str(e))

        return _dependencies_from_tuple(result)

    def nominate_cycle_breakers(self, package: str) -> set[ImportTuple]:
        """
        Identify a set of imports that, if removed, would make the package locally acyclic.
        """
        if not self._rustgraph.contains_module(package):
            raise ModuleNotPresent(f'"{package}" not present in the graph.')
        return self._rustgraph.nominate_cycle_breakers(package)

    # Dunder methods
    # --------------

    def __repr__(self) -> str:
        """
        Display the instance in one of the following ways:

            <ImportGraph: empty>
            <ImportGraph: 'one', 'two', 'three', 'four', 'five'>
            <ImportGraph: 'one', 'two', 'three', 'four', 'five', ...>
        """
        modules = self.modules
        if modules:
            repr_output_size = 5
            module_list = list(modules)[:repr_output_size]
            stringified_modules = ", ".join(repr(m) for m in module_list)
            if len(modules) > repr_output_size:
                stringified_modules += ", ..."
        else:
            stringified_modules = "empty"
        return f"<{self.__class__.__name__}: {stringified_modules}>"

    def __deepcopy__(self, memodict: dict) -> "ImportGraph":
        new_graph = ImportGraph()
        new_graph._rustgraph = self._rustgraph.clone()
        return new_graph


class _RustRoute(TypedDict):
    heads: frozenset[str]
    middle: tuple[str, ...]
    tails: frozenset[str]


class _RustPackageDependency(TypedDict):
    importer: str
    imported: str
    routes: tuple[_RustRoute, ...]


def _parse_layers(layers: Sequence[Layer | str | set[str]]) -> tuple[Layer, ...]:
    """
    Convert the passed raw `layers` into `Layer`s.
    """
    out_layers = []
    for layer in layers:
        if isinstance(layer, Layer):
            out_layers.append(layer)
        elif isinstance(layer, str):
            out_layers.append(Layer(layer))
        else:
            out_layers.append(Layer(*tuple(layer)))
    return tuple(out_layers)


def _dependencies_from_tuple(
    rust_package_dependency_tuple: tuple[_RustPackageDependency, ...],
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
