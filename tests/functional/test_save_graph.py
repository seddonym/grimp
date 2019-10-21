import tempfile
from pathlib import Path

import pytest
from grimp import build_graph, save_graph


"""
For ease of reference, these are the imports of all the files:

testpackage: None
testpackage.one: None
testpackage.one.alpha: sys, pytest
testpackage.one.beta: testpackage.one.alpha
testpackage.one.gamma: testpackage.one.beta
testpackage.one.delta: None
testpackage.one.delta.blue: None
testpackage.two: None:
testpackage.two.alpha: testpackage.one.alpha
testpackage.two.beta: testpackage.one.alpha
testpackage.two.gamma: testpackage.two.beta, testpackage.utils
testpackage.utils: testpackage.one, testpackage.two.alpha

"""


@pytest.mark.xfail
def test_save_graph():
    graph = build_graph("testpackage")
    filename = "grimp-graph.json"

    with tempfile.TemporaryDirectory() as temp_directory:
        full_filename = Path(temp_directory) / Path(filename)
        save_graph(graph, filename=full_filename)

        saved_contents = open(filename, "r").read()

    assert (
        saved_contents
        == open(Path("../assets/persistence/testpackage.json"), "r").read()
    )
