import pytest  # type: ignore

from grimp import build_graph

"""
For ease of reference, these are the imports of all the files:

mynamespace.yellow: mynamespace.green.alpha
mynamespace.green: mynamespace.yellow, functools
mynamespace.blue.alpha: mynamespace.blue.beta, mynamespace.green.alpha, itertools, urllib.request

And for the deeper namespace:

nestednamespace.foo.alpha.blue.one:
    pytest, itertools, urllib.request, nestednamespace.foo.alpha.blue.two
nestednamespace.foo.alpha.green.one:
    nestednamespace.foo.alpha.blue.one, nestednamespace.bar.beta.orange
"""

YELLOW_MODULES = {"mynamespace.yellow"}
GREEN_MODULES = {"mynamespace.green", "mynamespace.green.alpha"}
BLUE_MODULES = {"mynamespace.blue", "mynamespace.blue.alpha", "mynamespace.blue.beta"}


def test_build_graph_for_namespace():
    graph = build_graph("mynamespace", cache_dir=None)

    assert graph.modules == {"mynamespace"} | YELLOW_MODULES | GREEN_MODULES | BLUE_MODULES
    assert graph.count_imports()


@pytest.mark.parametrize(
    "package, expected_modules",
    (
        (
            "mynamespace.green",
            GREEN_MODULES,
        ),
        (
            "mynamespace.blue",
            BLUE_MODULES,
        ),
    ),
)
def test_modules_for_namespace_child(package, expected_modules):
    graph = build_graph(package, cache_dir=None)

    assert graph.modules == expected_modules


def test_modules_for_multiple_namespace_children():
    graph = build_graph("mynamespace.green", "mynamespace.blue", cache_dir=None)

    assert graph.modules == GREEN_MODULES | BLUE_MODULES


@pytest.mark.parametrize(
    "packages, expected_internal_modules, expected_external_modules",
    (
        (
            ("mynamespace.green",),
            GREEN_MODULES,
            {"functools", "mynamespace.yellow"},
        ),
        (
            ("mynamespace.blue",),
            BLUE_MODULES,
            {"itertools", "urllib", "mynamespace.green"},
        ),
        (
            ("mynamespace.green", "mynamespace.blue"),
            GREEN_MODULES | BLUE_MODULES,
            {"functools", "mynamespace.yellow", "itertools", "urllib"},
        ),
    ),
)
def test_external_packages_handling(
    packages, expected_internal_modules, expected_external_modules
):
    graph = build_graph(*packages, include_external_packages=True, cache_dir=None)

    assert graph.modules == expected_internal_modules | expected_external_modules
    assert all(graph.is_module_squashed(m) for m in expected_external_modules)


def test_import_within_namespace_child():
    graph = build_graph("mynamespace.blue", cache_dir=None)

    assert graph.direct_import_exists(
        importer="mynamespace.blue.alpha", imported="mynamespace.blue.beta"
    )


def test_import_between_namespace_children():
    graph = build_graph("mynamespace.blue", "mynamespace.green", cache_dir=None)

    assert graph.direct_import_exists(
        importer="mynamespace.blue.alpha", imported="mynamespace.green.alpha"
    )


# Nested namespaces

FOO_ALPHA_BLUE_MODULES = {
    "nestednamespace.foo.alpha.blue",
    "nestednamespace.foo.alpha.blue.one",
    "nestednamespace.foo.alpha.blue.two",
}
FOO_ALPHA_GREEN_MODULES = {
    "nestednamespace.foo.alpha.green",
    "nestednamespace.foo.alpha.green.one",
    "nestednamespace.foo.alpha.green.two",
}
BAR_BETA_MODULES = {
    "nestednamespace.bar.beta",
    "nestednamespace.bar.beta.orange",
}


@pytest.mark.parametrize(
    "package_name, expected",
    [
        (
            "nestednamespace",
            {
                "nestednamespace",
                "nestednamespace.foo",
                "nestednamespace.foo.alpha",
                "nestednamespace.bar",
            }
            | FOO_ALPHA_BLUE_MODULES
            | FOO_ALPHA_GREEN_MODULES
            | BAR_BETA_MODULES,
        ),
        (
            "nestednamespace.foo",
            {
                "nestednamespace.foo",
                "nestednamespace.foo.alpha",
            }
            | FOO_ALPHA_BLUE_MODULES
            | FOO_ALPHA_GREEN_MODULES,
        ),
        (
            "nestednamespace.foo.alpha",
            {"nestednamespace.foo.alpha"} | FOO_ALPHA_BLUE_MODULES | FOO_ALPHA_GREEN_MODULES,
        ),
    ],
)
def test_build_graph_for_nested_namespace(package_name, expected):
    graph = build_graph(package_name, cache_dir=None)

    assert graph.modules == expected
    assert graph.count_imports()


@pytest.mark.parametrize(
    "package, expected_modules",
    (
        (
            "nestednamespace.foo.alpha.green",
            {
                "nestednamespace.foo.alpha.green",
                "nestednamespace.foo.alpha.green.one",
                "nestednamespace.foo.alpha.green.two",
            },
        ),
        (
            "nestednamespace.foo.alpha.blue",
            {
                "nestednamespace.foo.alpha.blue",
                "nestednamespace.foo.alpha.blue.one",
                "nestednamespace.foo.alpha.blue.two",
            },
        ),
        (
            "nestednamespace.bar.beta",
            ({"nestednamespace.bar.beta", "nestednamespace.bar.beta.orange"}),
        ),
    ),
)
def test_modules_for_nested_namespace_child(package, expected_modules):
    graph = build_graph(package, cache_dir=None)

    assert graph.modules == expected_modules


def test_import_within_nested_namespace_child():
    graph = build_graph(
        "nestednamespace.foo.alpha.blue",
        cache_dir=None,
    )

    assert graph.direct_import_exists(
        importer="nestednamespace.foo.alpha.blue.one",
        imported="nestednamespace.foo.alpha.blue.two",
    )


def test_import_between_nested_namespace_children():
    graph = build_graph(
        "nestednamespace.foo.alpha.blue",
        "nestednamespace.foo.alpha.green",
        "nestednamespace.bar.beta",
        cache_dir=None,
    )

    assert graph.direct_import_exists(
        importer="nestednamespace.foo.alpha.green.one",
        imported="nestednamespace.foo.alpha.blue.one",
    )
    assert graph.direct_import_exists(
        importer="nestednamespace.foo.alpha.green.one",
        imported="nestednamespace.bar.beta.orange",
    )


@pytest.mark.parametrize(
    "packages, expected_internal_modules, expected_external_modules",
    (
        (
            ("nestednamespace.foo.alpha.blue",),
            # External packages generally resolve to the top level package.
            {
                "nestednamespace.foo.alpha.blue",
                "nestednamespace.foo.alpha.blue.one",
                "nestednamespace.foo.alpha.blue.two",
            },
            {
                "pytest",
                "itertools",
                "urllib",
            },
        ),
        (
            ("nestednamespace.foo.alpha.green",),
            # External packages that share a namespace with an internal module resolve
            # to the shallowest component that does not clash with an internal module namespace.
            {
                "nestednamespace.foo.alpha.green",
                "nestednamespace.foo.alpha.green.one",
                "nestednamespace.foo.alpha.green.two",
            },
            {
                "nestednamespace.foo.alpha.blue",
                "nestednamespace.bar",
            },
        ),
    ),
)
def test_external_packages_handling_for_nested_namespaces(
    packages, expected_internal_modules, expected_external_modules
):
    graph = build_graph(*packages, include_external_packages=True, cache_dir=None)

    assert graph.modules == expected_internal_modules | expected_external_modules
    assert all(graph.is_module_squashed(m) for m in expected_external_modules)
