import pytest
import json
import importlib
from pathlib import Path
from grimp.adaptors.graph import ImportGraph
import grimp


@pytest.fixture(scope="module")
def large_graph():
    raw_json = (Path(__file__).parent / "large_graph.json").read_text()
    graph_dict = json.loads(raw_json)
    graph = ImportGraph()

    for importer, importeds in graph_dict.items():
        graph.add_module(importer)
        for imported in importeds:
            graph.add_import(
                importer=importer,
                imported=imported,
                line_number=1,
                line_contents=f"import {imported}",
            )

    return graph


def test_build_django_uncached(benchmark):
    """
    Benchmarks building a graph of real package - in this case Django.

    In this benchmark, the cache is turned off.
    """
    benchmark(grimp.build_graph, "django", cache_dir=None)


def test_build_django_from_cache_no_misses(benchmark):
    """
    Benchmarks building a graph of real package - in this case Django.

    This benchmark fully utilizes the cache.
    """
    # Populate the cache first, before beginning the benchmark.
    grimp.build_graph("django")

    benchmark(grimp.build_graph, "django")


@pytest.mark.parametrize(
    "number_of_misses",
    (
        2,  # Fewer than the likely number of CPUs.
        15,  # A bit more than the likely number of CPUs.
    ),
)
def test_build_django_from_cache_a_few_misses(benchmark, number_of_misses):
    """
    Benchmarks building a graph of real package - in this case Django.

    This benchmark utilizes the cache except for a few modules, which we add.
    """
    # Populate the cache first, before beginning the benchmark.
    grimp.build_graph("django")
    # Add a module which won't be in the cache.
    django_path = Path(importlib.util.find_spec("django").origin).parent
    for i in range(number_of_misses):
        new_module = django_path / f"new_module_{i}.py"
        new_module.write_text("from django.db import models")

    benchmark(grimp.build_graph, "django")
