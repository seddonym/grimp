import abc
from typing import Set, Tuple, Optional, Dict, Union, List


class AbstractImportGraph(abc.ABC):
    """
    A Directed Graph of imports between Python modules.
    """
    # Mechanics
    # ---------

    @property
    @abc.abstractmethod
    def modules(self) -> Set[str]:
        """
        The names of all the modules in the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def add_module(self, module: str) -> None:
        """
        Add a module to the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def add_import(
            self, *,
            importer: str,
            imported: str,
            line_number: Optional[int] = None,
            line_contents: Optional[str] = None
    ) -> None:
        """
        Add a direct import between two modules to the graph. If the modules are not already
        present, they will be added to the graph.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def remove_import(self, *, importer: str, imported: str) -> None:
        """
        Remove a direct import between two modules. Does not remove the modules themselves.
        """
        raise NotImplementedError

    # Descendants
    # -----------

    @abc.abstractmethod
    def find_children(self, module: str) -> Set[str]:
        """
        Find all modules one level below the module. For example, the children of
        foo.bar might be foo.bar.one and foo.bar.two, but not foo.bar.two.green.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_descendants(self, module: str) -> Set[str]:
        """
        Find all modules below the module. For example, the descendants of
        foo.bar might be foo.bar.one and foo.bar.two and foo.bar.two.green.
        """
        raise NotImplementedError

    # Direct imports
    # --------------

    @abc.abstractmethod
    def direct_import_exists(self, *, importer: str, imported: str) -> bool:
        """
        Whether or not the importer module directly imports the imported module.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def find_modules_directly_imported_by(self, module: str) -> Set[str]:
        raise NotImplementedError

    @abc.abstractmethod
    def find_modules_that_directly_import(self, module: str) -> Set[str]:
        raise NotImplementedError

    @abc.abstractmethod
    def get_import_details(
        self,
        *,
        importer: str,
        imported: str
    ) -> List[Dict[str, Union[str, int]]]:
        """
        Returns a list of the details of every direct import between two modules, in the form:
        [
            {
                'importer': 'mypackage.importer',
                'imported': 'mypackage.imported',
                'line_number': 5,
                'line_contents': 'from mypackage import imported',
            },
            (additional imports here)
        ]
        """
        raise NotImplementedError

    # Indirect imports
    # ----------------

    @abc.abstractmethod
    def find_downstream_modules(
        self, module: str, as_package: bool = False
    ) -> Set[str]:
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
            import_graph.find_downstream_modules('mypackage.foo', as_package=True,)
        """
        raise NotImplementedError

    @abc.abstractmethod
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
        raise NotImplementedError

    @abc.abstractmethod
    def find_shortest_path(
        self, upstream_module: str, downstream_module: str,
    ) -> Optional[Tuple[str, ...]]:
        """
        Attempt to find the shortest ImportPath from the upstream to the downstream module.

        Returns:
            Tuple of module names, from importer to imported, or None if no path exists.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def path_exists(
            self, upstream_module: str, downstream_module: str, as_packages=False,
    ) -> bool:
        """
        Return whether any import path exists between the upstream and the downstream module,
        even indirectly; in other words, does the downstream module depend on the upstream module?

        Optional args:
            as_packages: Whether to treat the supplied modules as individual modules,
                         or as packages (including any descendants, if there are any). If
                         treating them as subpackages, all descendants of the upstream and
                         downstream modules will be checked too.
        """
        raise NotImplementedError
