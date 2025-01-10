import pytest
import json
from pathlib import Path
from grimp.adaptors.graph import ImportGraph
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


def test_build_django(benchmark):
    """
    Benchmarks building a graph of real package - in this case Django.
    """
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
            layers=("plugins", "application", "domain", "data"),
            containers=("mypackage",),
        )
    )


def test_deep_layers_large_graph(large_graph, benchmark):
    layers = (
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1991886645",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.6397984863",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.9009030339",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.6666171185",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1693068682",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1752284225",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.9089085203",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.5033127033",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.2454157946",
    )
    fn = lambda: large_graph.find_illegal_dependencies_for_layers(
        layers=layers,
    )
    if hasattr(benchmark, "pendantic"):
        # Running with pytest-benchmark
        benchmark.pedantic(fn, rounds=3)
    else:
        # Running with codspeed.
        benchmark(fn)
