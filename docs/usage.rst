=====
Usage
=====

Grimp provides an API in the form of an ``ImportGraph`` that represents all the internal imports within a
top-level Python package. This object has various methods that make it easy to find out information about
that package's structure and interdependencies.

Terminology
-----------

The terminology around Python packages and modules can be a little confusing. Here are the definitions we use,
taken in part from `the official Python docs`_:

- **Module**: A file containing Python definitions and statements. This includes ordinary ``.py`` files and
  ``__init__.py`` files.
- **Package**: A special kind of module that namespaces other modules using dotted module names. For example, the module
  name ``A.B`` designates a submodule named ``B`` in a package named ``A``. Packages take the form of ``__init__.py``
  files in a container directory. Packages may contain other packages. *A package is also a module.*
- **Top Level Package**: A package in the root namespace - in other words, one that is not a subpackage. For example,
  ``A`` is a top level package, but ``A.B`` is not.
- **Graph**: A graph `in the mathematical sense`_ of a collection of items with relationships between them. Grimp's
  ``ImportGraph`` is a directed graph of all the internal imports contained in a particular top level package.
- **Direct Import**: An import from one module to another.
- **Import Path**: A chain of imports between two modules, possibly via other modules. For example, if
  ``mypackage.foo`` imports ``mypackage.bar``, which in turn imports ``mypackage.baz``, then there is an import path
  between ``mypackage.foo`` and ``mypackage.baz``.

.. _the official Python docs: https://docs.python.org/3/tutorial/modules.html
.. _in the mathematical sense: https://en.wikipedia.org/wiki/Graph_(discrete_mathematics)

Building the graph
------------------

.. code-block:: python


    import grimp

    graph = grimp.build_graph('name_)

.. py:function:: grimp.build_graph(package_name)

   Build and return an ImportGraph for the supplied package.

    :param str package_name: The name of the top level package, for example ``'mypackage'``.
    :return: An import graph that you can use to analyse the package.
    :rtype: ImportGraph

Methods for analysing the module tree
-------------------------------------

.. py:attribute:: ImportGraph.modules

   All the modules in the top level package.

    :return: Set of module names.
    :rtype: A set of strings.

.. py:function:: ImportGraph.find_children(module)

   Return all the immediate children of the module, i.e. the modules that have a dotted module name that is one
   level below.

    :param str module: The importable name of the module, e.g. ``'mypackage'`` or ``'mypackage.foo.one'``. This may be
      any module within the package. It doesn't need to be a package itself, though if it isn't, it will have
      no children.
    :return: Set of module names.
    :rtype: A set of strings.

.. py:function:: ImportGraph.find_descendants(module)

   Return all the descendants of the module, i.e. the modules that have a dotted module name that is below
   the supplied module, to any depth.

    :param str module: The importable name of the module, e.g. ``'mypackage'`` or ``'mypackage.foo.one'``. As with
      ``find_children``, this doesn't have to be a package, though if it isn't then the set will be empty.
    :return: Set of module names.
    :rtype: A set of strings.

Methods for analysing direct imports
------------------------------------

.. py:function:: ImportGraph.direct_import_exists(importer, imported)

    :param str importer: A module name.
    :param str imported: A module name.
    :return: Whether or not the importer directly imports the imported module.
    :rtype: ``True`` or ``False``.

.. py:function:: ImportGraph.find_modules_directly_imported_by(module)

    :param str module: A module name.
    :return: Set of all modules in the graph are imported by the supplied module.
    :rtype: A set of strings.

.. py:function:: ImportGraph.find_modules_that_directly_import(module)

    :param str module: A module name.
    :return: Set of all modules in the graph that directly import the supplied module.
    :rtype: A set of strings.

.. py:function:: ImportGraph.get_import_details(importer, imported)

    Provides a way of seeing the details of direct imports between two modules (usually
    there will be only one of these, but it is possible for a module to import another
    module twice).

    The details are in the following form::

        [
            {
                'importer': 'mypackage.importer',
                'imported': 'mypackage.imported',
                'line_number': 5,
                'line_contents': 'from mypackage import imported',
            },
            # (additional imports here)
        ]

    :param str importer: A module name.
    :param str imported: A module name.
    :return: A list of the details of every direct import between two modules.
    :rtype: List of dictionaries.

Methods for analysing import paths
----------------------------------

.. py:function:: ImportGraph.find_downstream_modules(module, as_package=False)

    :param str module: A module name.
    :param bool as_package: Whether or not to treat the supplied module as an individual module,
                           or as an entire package (including any descendants). If
                           treating it as a package, the result will include downstream
                           modules *external* to the supplied module, and won't include modules within it.
    :return: All the modules that import (even indirectly) the supplied module.
    :rtype: A set of strings.

    Examples::

        # Returns the modules downstream of mypackage.foo.
        import_graph.find_downstream_modules('mypackage.foo')

        # Returns the modules downstream of mypackage.foo, mypackage.foo.one and
        # mypackage.foo.two.
        import_graph.find_downstream_modules('mypackage.foo', as_package=True)

.. py:function:: ImportGraph.find_upstream_modules(module, as_package=False)

    :param str module: A module name.
    :param bool as_package: Whether or not to treat the supplied module as an individual module,
                           or as a package (i.e. including any descendants, if there are any). If
                           treating it as a subpackage, the result will include upstream
                           modules *external* to the package, and won't include modules within it.
    :return: All the modules that are imported (even indirectly) by the supplied module.
    :rtype: A set of strings.

.. py:function:: ImportGraph.find_shortest_chain(upstream_module, downstream_module)

    :param str upstream_module: The module at the start of the potential import path (i.e. that the
        downstream module will import).
    :param str downstream_module: The module at the end of the potential import path (i.e. that will import
        the upstream module).
    :return: The shortest import path from the upstream to the downstream module,
             if one exists, or an empty tuple if not.
    :rtype: A tuple of strings, ordered from upstream to downstream modules.

.. py:function:: ImportGraph.chain_exists(upstream_module, downstream_module, as_packages=False)

    :param str upstream_module: The module at the start of the potential import path (i.e. that the
        downstream module will import).
    :param str downstream_module: The module at the end of the potential import path (i.e. that will import
        the upstream module).
    :param bool as_packages: Whether to treat the supplied modules as individual modules,
         or as packages (including any descendants, if there are any). If
         treating them as packages, all descendants of the upstream and
         downstream modules will be checked too.
    :return:  Return whether any import path exists between the upstream and the downstream module,
        even indirectly; in other words, does the downstream module depend on the upstream module?
    :rtype: bool

Methods for manipulating the graph
----------------------------------

.. py:function:: ImportGraph.add_module(module)

    Add a module to the graph.

    :param str module: The name of a module, for example ``'mypackage.foo'``.
    :return: None

.. py:function:: ImportGraph.add_import(importer, imported, line_number=None, line_contents=None)

    Add a direct import between two modules to the graph. If the modules are not already
    present, they will be added to the graph.

    :param str importer: The name of the module that is importing the other module.
    :param str imported: The name of the module being imported.
    :param int line_number: The line number of the import statement in the module.
    :param str line_contents: The line that contains the import statement.
    :return: None

.. py:function:: ImportGraph.remove_import(importer, imported)

    Remove a direct import between two modules. Does not remove the modules themselves.

    :param str importer: The name of the module that is importing the other module.
    :param str imported: The name of the module being imported.
    :return: None
