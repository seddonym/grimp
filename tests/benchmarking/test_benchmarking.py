import pytest
import json
from pathlib import Path
from grimp.adaptors.graph import ImportGraph
from grimp import PackageDependency, Route
import grimp


@pytest.fixture(scope="module")
def large_graph():
    raw_json = (Path(__file__).parent / "large_graph.json").read_text()
    graph_dict = json.loads(raw_json)
    graph = ImportGraph()

    for importer, importeds in graph_dict.items():
        for imported in importeds:
            graph.add_import(importer=importer, imported=imported)

    return graph


TOP_LEVEL_LAYERS = ("plugins", "application", "domain", "data")
DEEP_PACKAGE = "mypackage.plugins.5634303718.1007553798.8198145119"
DEEP_LAYERS = (
    f"{DEEP_PACKAGE}.application.3242334296.1991886645",
    f"{DEEP_PACKAGE}.application.3242334296.6397984863",
    f"{DEEP_PACKAGE}.application.3242334296.9009030339",
    f"{DEEP_PACKAGE}.application.3242334296.6666171185",
    f"{DEEP_PACKAGE}.application.3242334296.1693068682",
    f"{DEEP_PACKAGE}.application.3242334296.1752284225",
    f"{DEEP_PACKAGE}.application.3242334296.9089085203",
    f"{DEEP_PACKAGE}.application.3242334296.5033127033",
    f"{DEEP_PACKAGE}.application.3242334296.2454157946",
)


def test_build_django_uncached(benchmark):
    """
    Benchmarks building a graph of real package - in this case Django.

    In this benchmark, the cache is turned off.
    """
    fn = lambda: grimp.build_graph("django", cache_dir=None)
    if hasattr(benchmark, "pendantic"):
        # Running with pytest-benchmark
        benchmark.pedantic(fn, rounds=3)
    else:
        # Running with codspeed.
        benchmark(fn)


def test_build_django_from_cache(benchmark):
    """
    Benchmarks building a graph of real package - in this case Django.

    This benchmark uses the cache.
    """
    # Populate the cache first, before beginning the benchmark.
    grimp.build_graph("django")

    fn = lambda: grimp.build_graph("django")
    if hasattr(benchmark, "pendantic"):
        # Running with pytest-benchmark
        benchmark.pedantic(fn, rounds=3)
    else:
        # Running with codspeed.
        benchmark(fn)


def test_top_level_large_graph(large_graph, benchmark):
    benchmark(
        lambda: large_graph.find_illegal_dependencies_for_layers(
            layers=TOP_LEVEL_LAYERS,
            containers=("mypackage",),
        )
    )


def test_deep_layers_large_graph(large_graph, benchmark):
    fn = lambda: large_graph.find_illegal_dependencies_for_layers(layers=DEEP_LAYERS)
    if hasattr(benchmark, "pendantic"):
        # Running with pytest-benchmark
        benchmark.pedantic(fn, rounds=3)
    else:
        # Running with codspeed.
        benchmark(fn)


# Result checks
# -------------
# These tests aren't benchmarks, but they execute the same code as the benchmarks to check the
# behaviour hasn't changed.


def test_top_level_large_graph_result_check(large_graph):
    result = large_graph.find_illegal_dependencies_for_layers(
        layers=TOP_LEVEL_LAYERS,
        containers=("mypackage",),
    )

    assert result == set()


def test_deep_layers_large_graph_result_check(large_graph):
    result = large_graph.find_illegal_dependencies_for_layers(layers=DEEP_LAYERS)
    assert result == {
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.2454157946",
            imported=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset({f"{DEEP_PACKAGE}.application.3242334296.2454157946"}),
                        middle=(),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.9089085203"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.5033127033",
            imported=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset({f"{DEEP_PACKAGE}.application.3242334296.5033127033"}),
                        middle=(),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.9089085203"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.1693068682",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.1693068682"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.1752284225",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.1752284225"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.1991886645",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.1991886645"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.6397984863",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.6397984863"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.6666171185",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.6666171185"}),
                    )
                }
            ),
        ),
        PackageDependency(
            importer=f"{DEEP_PACKAGE}.application.3242334296.9089085203",
            imported=f"{DEEP_PACKAGE}.application.3242334296.9009030339",
            routes=frozenset(
                {
                    Route(
                        heads=frozenset(
                            {
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4296536723",
                                f"{DEEP_PACKAGE}.application.3242334296.9089085203.4641062780",
                            }
                        ),
                        middle=(f"{DEEP_PACKAGE}.application.3242334296",),
                        tails=frozenset({f"{DEEP_PACKAGE}.application.3242334296.9009030339"}),
                    )
                }
            ),
        ),
    }
