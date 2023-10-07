=======
Caching
=======

Grimp uses a file-based cache to speed up subsequent builds of the graph::

    >>> build_graph("somepackage", "anotherpackage")  # Writes to a cache for the first time.
    ...
    >>> build_graph("somepackage", "anotherpackage")  # Second time it's run, it's much quicker.

What is cached?
---------------

Grimp caches the imports discovered through static analysis of the packages when it builds a graph.
It does not cache the results of any methods called on a graph, e.g. ``find_downstream_modules``.

Separate caches of imports are created depending on the *set of packages* passed to ``build_graph``,
together with whether or not the graph should include external packages.

For example, the following invocations will each have a separate cache and will not
be able to make use of each other's work:

- ``build_graph("mypackage")``
- ``build_graph("mypackage", "anotherpackage")``
- ``build_graph("mypackage", "anotherpackage", include_external_packages=True)``
- ``build_graph("mypackage", "anotherpackage", exclude_type_checking_imports=True)``

Grimp can make use of cached results even if some of the modules change. For example,
if ``mypackage.foo`` is changed, but all the other modules within ``mypackage`` are left
untouched, Grimp will only need to rescan ``mypackage.foo``. This can have a significant
speed up effect when analysing large codebases in which only a small subset of files change
from run to run.

Grimp determines whether or not it needs to rescan a file based on its last modified time.
This makes it very effective for local development, but is less effective in environments
that reinstall the package under analysis between each build of the graph (e.g. on a
continuous integration server).

Location of the cache
---------------------

Cache files are written, by default, to a ``.grimp_cache`` directory
in the current working directory. This directory can be changed by passing
``cache_dir`` to the ``build_graph`` function, e.g.::

    graph = grimp.build_graph("mypackage", cache_dir="/path/to/cache")

Disabling caching
-----------------

To skip using (and writing to) the cache, pass ``cache_dir=None`` to ``build_graph``::

    graph = grimp.build_graph("mypackage", cache_dir=None)

Concurrency
-----------

Caching isn't currently concurrency-safe. Specifically, if you have two concurrent processes writing to the same cache
files, you might experience incorrect behaviour.
