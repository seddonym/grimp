========
NetworkX
========

If you want to analyze the graph in a way that isn't provided by Grimp, you may want to consider converting the graph to a `NetworkX`_ graph.

NetworkX is a third-party Python library with a large number of algorithms for working with graphs.

Converting the Grimp graph to a NetworkX graph
----------------------------------------------

First, you should install NetworkX (e.g. ``pip install networkx``).

You can then build up a NetworkX graph as shown::

    import grimp
    import networkx

    grimp_graph = grimp.build_graph("mypackage")

    # Build a NetworkX graph from the Grimp graph.
    networkx_graph = networkx.DiGraph()
    for module in grimp_graph.modules:
        networkx_graph.add_node(module)
        for imported in grimp_graph.find_modules_directly_imported_by(module):
            networkx_graph.add_edge(module, imported)

.. _NetworkX: https://networkx.org/