from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Route:
    """
    A set of 'chains' that share the same middle.

    A chain is a sequence of modules linked by imports, for example:
    mypackage.foo -> mypackage.bar -> mypackage.baz.

    The route fans in at the head and out at the tail, but the middle of the chain just links
    individual modules.
    """

    heads: frozenset[str]  # Importer modules at the start of the chain.
    middle: tuple[str, ...]
    tails: frozenset[str]  # Imported modules at the end of the chain.


@dataclass(frozen=True)
class PackageDependency:
    """
    Dependencies from one package to another.
    """

    # The full name of the package from which all the routes start.
    upstream: str
    # The full name of the package from which all the routes end.
    downstream: str
    routes: frozenset[Route]
