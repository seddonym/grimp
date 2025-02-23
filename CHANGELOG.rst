
Changelog
=========

Unreleased
----------

* Added basic functions for working with module expressions: `find_matching_modules` and `find_matching_direct_imports`.
* Added `as_packages` option to the `find_shortest_chain` method.

3.6 (2025-02-07)
----------------

* Reimplement the graph in Rust. This is a substantial rewrite that, mostly, significantly
  improves performance, but there may be certain operations that are slower than before.

3.5 (2024-10-08)
----------------

* Added `as_packages` option to the `find_shortest_chains` method.
* Include 3.13 wheel in release.
* Drop support for Python 3.8.

3.4.1 (2024-07-12)
------------------

* Officially support Python 3.13.

3.4 (2024-07-09)
----------------

* Speed up adding and removing modules.

3.3 (2024-06-13)
----------------

* Upgrade PyO3 to 0.21.
* Follow symbolic links while walking through module files.
* Speed up find_illegal_dependencies_for_layers by using thread concurrency.

3.2 (2024-1-8)
--------------

* Allow configuring sibling layer independence.
* Fix bug where a warning would be logged if a non-Python file with multiple dots
  in the filename was encountered.
* Formally add support for Python 3.12.

3.1 (2023-10-13)
----------------

* Add exclude_type_checking_imports argument to build_graph.

3.0 (2023-8-18)
---------------

* Stable release of functionality from 3.0b1-3.

3.0b3 (2023-8-17)
-----------------

* Support for independent layers in find_illegal_dependencies_for_layers.

3.0b1, 3.0b2 (2023-8-15)
------------------------

* Switch to pyproject.toml.
* Rename upstream/downstream in find_illegal_dependencies_for_layers to importer/imported.
  The original names were accidentally used in reverse; the new names have less potential for confusion.
* Use Rust extension module for find_illegal_dependencies_for_layers.

2.5 (2023-7-6)
--------------

* Log cache activity.
* Drop support for Python 3.7.
* Add find_illegal_dependencies_for_layers method.

2.4 (2023-5-5)
--------------

* Change cache filename scheme to use a hash.
* Ignore modules with dots in the filename.

2.3 (2023-3-3)
--------------

* Add caching.

2.2 (2023-1-5)
--------------

* Annotate get_import_details return value with a DetailedImport.

2.1 (2022-12-2)
---------------

* Officially support Python 3.11.

2.0 (2022-9-27)
---------------

* Significantly speed up graph copying.
* Remove find_all_simple_chains method.
* No longer use a networkx graph internally.
* Fix bug where import details remained stored in the graph after removing modules or imports.

1.3 (2022-8-15)
---------------
* Officially support Python 3.9 and 3.10.
* Drop support for Python 3.6.
* Support namespaced packages.

1.2.3 (2021-1-19)
-----------------
* Raise custom exception (NamespacePackageEncountered) if code under analysis appears to be a namespace package.

1.2.2 (2020-6-29)
-----------------
* Raise custom exception (SourceSyntaxError) if code under analysis contains syntax error.

1.2.1 (2020-3-16)
-----------------
* Better handling of source code containing non-ascii compatible characters

1.2 (2019-11-27)
----------------
* Significantly increase the speed of building the graph.

1.1 (2019-11-18)
----------------
* Clarify behaviour of get_import_details.
* Add module_is_squashed method.
* Add squash_module method.
* Add find_all_simple_chains method.

1.0 (2019-10-17)
----------------
* Officially support Python 3.8.

1.0b13 (2019-9-25)
------------------
* Support multiple root packages.

1.0b12 (2019-6-12)
------------------
* Add find_shortest_chains method.

1.0b11 (2019-5-18)
------------------
* Add remove_module method.

1.0b10 (2019-5-15)
------------------
* Fix Windows incompatibility.

1.0b9 (2019-4-16)
-----------------
* Fix bug with calling importlib.util.find_spec.

1.0b8 (2019-2-1)
----------------
* Add as_packages parameter to direct_import_exists.

1.0b7 (2019-1-21)
-----------------
* Add count_imports method.

1.0b6 (2019-1-20)
-----------------
* Support building the graph with external packages.

1.0b5 (2019-1-12)
-----------------
* Rename get_shortest_path to get_shortest_chain.
* Rename path_exists to chain_exists.
* Rename and reorder the kwargs for get_shortest_chain and chain_exists.
* Raise ValueError if modules with shared descendants are passed to chain_exists if as_packages=True.

1.0b4 (2019-1-7)
----------------
* Improve repr of ImportGraph.
* Fix bug with find_shortest_path using upstream/downstream the wrong way around.

1.0b3 (2018-12-16)
------------------
* Fix bug with analysing relative imports from within __init__.py files.
* Stop skipping analysing packages called ``migrations``.
* Deal with invalid imports by warning instead of raising an exception.
* Rename NetworkXBackedImportGraph to ImportGraph.

1.0b2 (2018-12-12)
------------------
* Fix PyPI readme rendering.

1.0b1 (2018-12-08)
------------------
* Implement core functionality.

0.0.1 (2018-11-05)
------------------
* Release blank project on PyPI.
