from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, Sequence


@dataclass(frozen=True)
class Route:
    """
    A set of 'chains' that share the same middle.

    A chain is a sequence of modules linked by imports, from importer to imported,
    for example:
    mypackage.foo -> mypackage.bar -> mypackage.baz.

    The route fans in at the head and out at the tail, but the middle of the chain just links
    individual modules.
    """

    heads: frozenset[str]  # Importer modules at the start of the chain.
    middle: tuple[str, ...]
    tails: frozenset[str]  # Imported modules at the end of the chain.

    @classmethod
    def new(
        cls,
        heads: Iterable[str],
        tails: Iterable[str],
        middle: Sequence[str] | None = None,
    ) -> Route:
        """
        Optional constructor for a Route with more permissive input types.

        Example:

            Route.new(
                heads={"foo"},
                middle=["bar", "baz"],
                tails={"foobar"},
            )

        """
        return cls(
            heads=frozenset(heads),
            middle=tuple(middle) if middle else (),
            tails=frozenset(tails),
        )

    @classmethod
    def single_chained(cls, *modules: str) -> Route:
        """
        Optional constructor for a Route with a single chain.

        Example:

            Route.single_chained("foo", "bar", "baz")

        """
        return Route.new(
            heads={modules[0]},
            middle=tuple(modules[1:-1]),
            tails={modules[-1]},
        )


@dataclass(frozen=True)
class PackageDependency:
    """
    Dependencies from one package to another.
    """

    # The full name of the package from which all the routes start;
    # the downstream package.
    importer: str
    # The full name of the package from which all the routes end;
    # the upstream package.
    imported: str

    routes: frozenset[Route]

    @classmethod
    def new(
        cls,
        importer: str,
        imported: str,
        routes: Iterable[Route],
    ) -> PackageDependency:
        """
        Optional constructor for a PackageDependency with more permissive input types.

        Example:

            PackageDependency.new(
                importer="foo",
                imported="bar",
                routes={Route.single_chained("foo", "bar")},
            )

        """
        return cls(importer=importer, imported=imported, routes=frozenset(routes))
