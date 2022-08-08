=====
Grimp
=====

.. image:: https://img.shields.io/pypi/v/grimp.svg
    :target: https://pypi.org/project/grimp

.. image:: https://img.shields.io/pypi/pyversions/grimp.svg
    :alt: Python versions
    :target: https://pypi.org/project/grimp/

.. image:: https://api.travis-ci.com/seddonym/grimp.svg?branch=master
    :target: https://app.travis-ci.com/github/seddonym/grimp


Builds a queryable graph of the imports within one or more Python packages.

* Free software: BSD license

Quick start
-----------

Install grimp::

    pip install grimp

Install the Python package you wish to analyse::

    pip install somepackage

In Python, build the import graph for the package::

    >>> import grimp
    >>> graph = grimp.build_graph('somepackage')

You may now use the graph object to analyse the package. Some examples::

    >>> graph.find_children('somepackage.foo')
    {
        'somepackage.foo.one',
        'somepackage.foo.two',
    }

    >>> graph.find_descendants('somepackage.foo')
    {
        'somepackage.foo.one',
        'somepackage.foo.two',
        'somepackage.foo.two.blue',
        'somepackage.foo.two.green',
    }

    >>> graph.find_modules_directly_imported_by('somepackage.foo')
    {
        'somepackage.bar.one',
    }

    >>> graph.find_upstream_modules('somepackage.foo')
    {
        'somepackage.bar.one',
        'somepackage.baz',
        'somepackage.foobar',
    }

    >>> graph.find_shortest_chain(importer='somepackage.foobar', imported='somepackage.foo')
    (
        'somepackage.foobar',
        'somepackage.baz',
        'somepackage.foo',
    )

    >>> graph.get_import_details(importer='somepackage.foobar', imported='somepackage.baz'))
    [
        {
            'importer': 'somepackage.foobar',
            'imported': 'somepackage.baz',
            'line_number': 5,
            'line_contents': 'from . import baz',
        },
    ]

For the full list of methods, see :doc:`usage`.

External packages
-----------------

By default, external dependencies will not be included. This can be overridden like so::

    >>> graph = grimp.build_graph('somepackage', include_external_packages=True)
    >>> graph.find_modules_directly_imported_by('somepackage.foo')
    {
        'somepackage.bar.one',
        'os',
        'decimal',
        'sqlalchemy',
    }

Multiple packages
-----------------

You may analyse multiple root packages. To do this, pass each package name as a positional argument::

    >>> graph = grimp.build_graph('somepackage', 'anotherpackage')
    >>> graph.find_modules_directly_imported_by('somepackage.foo')
    {
        'somepackage.bar.one',
        'anotherpackage.baz',
    }

Namespace packages
------------------

Grimp supports `native namespace packages`_.

This Python feature allows subpackages to be distributed independently, while still importable under a shared namespace. `The Python client for Google's Cloud Logging API`_, for example, makes us of this. When installed, it is importable in Python as ``google.cloud.logging``. The parent packages ``google`` and ``google.cloud`` are the namespace packages, while ``google.cloud.logging`` is the distribution package. Other distribution packages can be installed separately that are also importable under those namespaces.

Grimp expects the package name passed to ``build_graph`` to be a distribution package, rather than a namespace package. So in the case of the example above, the graph should be built like so:

    >>> graph = grimp.build_graph('google.cloud.logging')

If, instead, a namespace package is passed (e.g. ``grimp.build_graph('google.cloud')``), Grimp will raise ``NamespacePackageEncountered``.

.. _native namespace packages: https://packaging.python.org/en/latest/guides/packaging-namespace-packages/#native-namespace-packages
.. _The Python client for Google's Cloud Logging API: https://pypi.org/project/google-cloud-logging/