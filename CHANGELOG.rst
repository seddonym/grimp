
Changelog
=========

0.0.1 (2018-11-05)
------------------

* Release blank project on PyPI.

1.0b1 (2018-12-08)
------------------

* Implement core functionality.

1.0b2 (2018-12-12)
------------------

* Fix PyPI readme rendering.

1.0b3 (2018-12-16)
------------------

* Fix bug with analysing relative imports from within __init__.py files.
* Stop skipping analysing packages called ``migrations``.
* Deal with invalid imports by warning instead of raising an exception.
* Rename NetworkXBackedImportGraph to ImportGraph.

1.0b4 (2019-1-7)
----------------

* Improve repr of ImportGraph.
* Fix bug with find_shortest_path using upstream/downstream the wrong way around.

1.0b5 (2019-1-12)
-----------------
* Rename get_shortest_path to get_shortest_chain.
* Rename path_exists to chain_exists.
* Rename and reorder the kwargs for get_shortest_chain and chain_exists.
* Raise ValueError if modules with shared descendants are passed to chain_exists if as_packages=True.

1.0b6 (2019-1-20)
-----------------
* Support building the graph with external packages.

1.0b7 (2019-1-21)
-----------------
* Add count_imports method.

1.0b8 (2019-2-1)
----------------
* Add as_packages parameter to direct_import_exists.

1.0b9 (2019-4-16)
-----------------
* Fix bug with calling importlib.util.find_spec.

1.0b10 (2019-5-15)
------------------
* Fix Windows incompatibility.

1.0b11 (2019-5-18)
------------------
* Add remove_module method.
