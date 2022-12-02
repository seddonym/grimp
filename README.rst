=====
Grimp
=====

.. image:: https://img.shields.io/pypi/v/grimp.svg
    :target: https://pypi.org/project/grimp

.. image:: https://img.shields.io/pypi/pyversions/grimp.svg
    :alt: Python versions
    :target: https://pypi.org/project/grimp/

.. image:: https://github.com/seddonym/grimp/workflows/CI/badge.svg?branch=master
     :target: https://github.com/seddonym/grimp/actions?workflow=CI
     :alt: CI Status

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

Graphs can also be built from `portions`_ of `namespace packages`_. To do this, provide the portion name, rather than the namespace name:

    >>> graph = grimp.build_graph('somenamespace.foo')

What's a namespace package?
###########################

Namespace packages are a Python feature allows subpackages to be distributed independently, while still importable under a shared namespace. This is, for example, used by `the Python client for Google's Cloud Logging API`_. When installed, it is importable in Python as ``google.cloud.logging``. The parent packages ``google`` and ``google.cloud`` are both namespace packages, while ``google.cloud.logging`` is known as the 'portion'. Other portions in the same namespace can be installed separately, for example ``google.cloud.secretmanager``.

Grimp expects the package name passed to ``build_graph`` to be a portion, rather than a namespace package. So in the case of the example above, the graph should be built like so:

    >>> graph = grimp.build_graph('google.cloud.logging')

If, instead, a namespace package is passed (e.g. ``grimp.build_graph('google.cloud')``), Grimp will raise ``NamespacePackageEncountered``.

.. _portions: https://docs.python.org/3/glossary.html#term-portion
.. _namespace packages: https://docs.python.org/3/glossary.html#term-namespace-package
.. _The Python client for Google's Cloud Logging API: https://pypi.org/project/google-cloud-logging/