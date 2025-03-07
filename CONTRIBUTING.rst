============
Contributing
============

Contributions are welcome, and they are greatly appreciated! Every
little bit helps, and credit will always be given.

Bug reports
===========

When `reporting a bug <https://github.com/seddonym/grimp/issues>`_ please include:

    * Your operating system name and version.
    * Any details about your local setup that might be helpful in troubleshooting.
    * Detailed steps to reproduce the bug.

Documentation improvements
==========================

Grimp could always use more documentation, whether as part of the
official docs, in docstrings, or even on the web in blog posts,
articles, and such.

Feature requests and feedback
=============================

The best way to send feedback is to file an issue at https://github.com/seddonym/grimp/issues.

If you are proposing a feature:

* Explain in detail how it would work.
* Keep the scope as narrow as possible, to make it easier to implement.
* Remember that this is a volunteer-driven project, and that code contributions are welcome :)

Development
===========

To set up `grimp` for local development:

1. Fork `grimp <https://github.com/seddonym/grimp>`_
   (look for the "Fork" button).
2. Clone your fork locally::

    git clone git@github.com:your_name_here/grimp.git

3. Create a branch for local development::

    git checkout -b name-of-your-bugfix-or-feature

   Now you can make your changes locally.

4. When you're done making changes, run all the checks, doc builder and spell checker with `tox <https://tox.wiki/en/stable/installation.html>`_ one command::

    tox

5. Commit your changes and push your branch to GitHub::

    git add .
    git commit -m "Your detailed description of your changes."
    git push origin name-of-your-bugfix-or-feature

6. Submit a pull request through the GitHub website.

Rust code
---------

When working with the rust code (in the ``rust/`` directory):

* Run tests with ``cargo test --no-default-features``.
  The ``--no-default-features`` flag is needed to due to `this PYO3 issue <https://pyo3.rs/main/faq#i-cant-run-cargo-test-or-i-cant-build-in-a-cargo-workspace-im-having-linker-issues-like-symbol-not-found-or-undefined-reference-to-_pyexc_systemerror>`_.
* Run `clippy <https://doc.rust-lang.org/clippy/index.html>`_ (a linter for rust) with ``cargo clippy --all-targets --all-features -- -D warnings``.
  It's often possible to apply automatic fixes to clippy issues with the ``--fix`` flag e.g. ``cargo clippy --all-targets --all-features --fix --allow-staged``.

Pull Request Guidelines
-----------------------

If you need some code review or feedback while you're developing the code just make the pull request.

For merging, you should:

1. Include passing tests (run ``tox``) [1]_.
2. Update documentation when there's new API, functionality etc.
3. Add a note to ``CHANGELOG.rst`` about the changes.
4. Add yourself to ``AUTHORS.rst``.

.. [1] If you don't have all the necessary python versions available locally you can rely on Github Actions, which will
       run the tests for each change you add in the pull request.

       It will be slower though ...

Tips
----

To run a subset of tests::

    tox -e envname -- pytest -k test_myfeature

To run all the test environments in *parallel* (you need to ``pip install detox``)::

    detox


Benchmarking
============

Codspeed
--------

A few benchmarks are run automatically on pull requests, using `Codspeed <https://codspeed.io/>`_.
Once the benchmarks have completed, a report will be included as a comment on the pull request.

Codspeed also shows flame graphs which can help track down why a change might have impacted performance.

Local benchmarking
------------------

It's also possible to run local benchmarks, which can be helpful if you want to quickly compare performance
across different versions of the code.

To benchmark a particular version of the code, run ``tox -ebenchmark``. This command creates a report that will be
stored in a local file (ignored by Git).

You can then see how your latest benchmark compares with earlier ones, by running:

``pytest-benchmark compare --group-by=func --sort=name --columns=mean``

This will display a list of all the benchmarks you've run locally, ordered from earlier to later.

Profiling
=========

Codspeed
--------

The easiest way to profile code is to look at the Codspeed flamegraph, automatically generated during benchmarking
(see above).

Profiling Rust code locally
---------------------------

Rust integration tests can be profiled using `Cargo Flamegraph <https://github.com/flamegraph-rs/flamegraph>`_
(which will need to be installed first, e.g. using ``cargo install flamegraph``).

Navigate to the ``rust`` directory in this package.

Run cargo flamegraph on the relevant test. E.g. to profile ``rust/tests/large.rs``, run:

``sudo cargo flamegraph --root --test large``

This will create a file called ``flamegraph.svg``, which you can open to view the flamegraph.

Releasing to Pypi
=================

1. Choose a new version number (based on `semver <https://semver.org/>`_).
2. ``git pull origin master``
3. Update ``CHANGELOG.rst`` with the new version number.
4. Update the ``release`` variable in ``docs/conf.py`` with the new version number.
5. Update the ``__version__`` variable in ``src/grimp/__init__.py` with the new version number.
6. Update ``project.version`` in ``pyproject.toml`` with the new version number.
7. ``git commit -am "Release v{new version number"``
8. ``git push``
9. Wait for tests to pass on CI.
10. ``git tag v{new version number}``
11. ``git push --tags``
12. This should kick start the Github ``release`` workflow, which builds wheels and releases the
    project to PyPI.
