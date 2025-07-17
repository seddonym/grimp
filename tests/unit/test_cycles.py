"""
TODO K4liber: 
- remove this notes
- add more test cases (real-life libraries?)

To rebuild the rust binary run the following command from the root directory:

python -m pip install -e .

"""

from grimp.adaptors.graph import ImportGraph


class TestFindCycles:

    def test_empty_graph(self) -> None:
        graph = ImportGraph()
        graph.add_module("A")
        graph.add_module("B")
        graph.add_module("C")
        graph.add_import(importer="A", imported="B")
        graph.add_import(importer="B", imported="C")
        graph.add_import(importer="C", imported="A")
        cycles = graph.find_cycles()
        assert cycles == [["A", "B", "C"]]
