import abc
from typing import Set, Optional

from grimp.domain.valueobjects import DirectImport, Module, ImportPath


class AbstractImportGraph(abc.ABC):
    """
    A Directed Graph of imports between Python modules.
    """
    @property
    @abc.abstractmethod
    def modules(self) -> Set[Module]:
        """
        All the modules in the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_modules_directly_imported_by(self, module: Module) -> Set[Module]:
        raise NotImplementedError

    @abc.abstractmethod
    def find_modules_that_directly_import(self, module: Module) -> Set[Module]:
        raise NotImplementedError

    @abc.abstractmethod
    def find_downstream_modules(
        self, module: Module, as_subpackage: bool = False
    ) -> Set[Module]:
        """
        Return a set of all the modules that import (even indirectly) the supplied module.
        Args:
            module:        The upstream Module.
            as_subpackage: Whether or not to treat the supplied module as an individual module,
                           or as an entire subpackage (including any descendants). If
                           treating it as a subpackage, the result will include downstream
                           modules *external* to the subpackage, and won't include modules within
                           the subpackage.
        Usage:
            # Returns the modules downstream of mypackage.foo.
            import_graph.find_downstream_modules(
                Module('mypackage.foo'),
            )
            # Returns the modules downstream of mypackage.foo, mypackage.foo.one and
            mypackage.foo.two.
            import_graph.find_downstream_modules(
                Module('mypackage.foo'),
                as_subpackage=True,
            )
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_upstream_modules(
        self, module: Module, as_subpackage: bool = False
    ) -> Set[Module]:
        """
        Return a set of all the modules that are imported (even indirectly) by the supplied module.

        Args:
            module:        The downstream Module.
            as_subpackage: Whether or not to treat the supplied module as an individual module,
                           or as an entire subpackage (including any descendants). If
                           treating it as a subpackage, the result will include upstream
                           modules *external* to the subpackage, and won't include modules within
                           the subpackage.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_children(self, module: Module) -> Set[Module]:
        """
        Find all modules one level below the module. For example, the children of
        foo.bar might be foo.bar.one and foo.bar.two, but not foo.bar.two.green.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_descendants(self, module: Module) -> Set[Module]:
        """
        Find all modules below the module. For example, the descendants of
        foo.bar might be foo.bar.one and foo.bar.two and foo.bar.two.green.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_shortest_path(
        self, upstream_module: Module, downstream_module: Module,
    ) -> Optional[ImportPath]:
        """
        Attempt to find the shortest ImportPath from the upstream to the downstream module.

        Returns:
            ImportPath, or None if no path could be found.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def add_module(self, module: Module) -> None:
        """
        Add a module to the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def add_import(self, direct_import: DirectImport) -> None:
        """
        Add a direct import between two modules to the graph. If the modules are not already
        present, they will be added to the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def remove_import(self, direct_import: DirectImport) -> None:
        """
        Remove a direct import between two modules. Does not remove the modules themselves.
        """
        raise NotImplementedError
