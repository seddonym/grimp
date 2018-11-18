"""
Use cases handle application logic.
"""
from typing import Optional

from ..application.ports.graph import AbstractImportGraph
from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.modulefinder import AbstractModuleFinder
from ..application.ports.importscanner import AbstractImportScanner
from ..application.ports.packagefinder import AbstractPackageFinder
from ..domain.valueobjects import Module

from .config import settings


def build_graph(package_name) -> AbstractImportGraph:
    """
    Build and return an import graph for the supplied package name.
    """
    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    file_system: AbstractFileSystem = settings.FILE_SYSTEM
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    package_directory = package_finder.determine_package_directory(
        package_name=package_name,
        file_system=file_system,
    )

    # Build a list of all the Python modules in the package.
    modules = module_finder.find_modules(
        package_name=package_name,
        package_directory=package_directory,
        file_system=file_system,
    )

    import_scanner: AbstractImportScanner = settings.IMPORT_SCANNER_CLASS(
        modules=modules,
        package_directory=package_directory,
        file_system=file_system,
    )

    graph: AbstractImportGraph = settings.IMPORT_GRAPH_CLASS()

    # Scan each module for imports and add them to the graph.
    for module in modules:
        graph.add_module(module)
        for direct_import in import_scanner.scan_for_imports(module):
            graph.add_import(direct_import)

    return graph


def draw_graph(module_name: str, filename: str):
    """
    It would be nice to be able to say any of these:
        - Draw me a complete graph for foo.
        - Draw me a graph of foo.one.
        - Draw me a graph of foo but only n levels deep.
        - Include external deps, n levels deep.
    Would be good if the graphviz clusters the first level of subpackages.

    This is all pointing towards being able to transform the graph in different ways, then
    pass the graph to draw_graph.
    """
    try:
        from graphviz import Digraph
    except ImportError:
        raise RuntimeError('Graphviz must be installed. Try pip install graphviz.')

    module = Module(module_name)
    graph = build_graph(module.package_name)
    module_children = graph.find_children(module)

    dot = Digraph(filename=filename)
    for module_child in module_children:
        dot.node(module_child.name)

    for direct_import in graph.direct_imports:
        dot.edge(direct_import.importer.name, direct_import.imported.name)
    # dot.render(filename, view=True)
    dot.view()

# def draw_graph(filename: str, graph: AbstractImportGraph):
#     from graphviz import Digraph
#
#     g = Digraph('G', filename=filename)
#     g.attr(
#         # diredgeconstraints='true',
#         compound='true',
#         newrank='true',
#         ranksep='4',
#     )
#
#     root = Module('grimp')
#     children = graph.find_children(root)
#     selected_modules = {root}
#     for module in children:
#         with g.subgraph(name=f'cluster_{module}') as cluster:
#             print(f'Cluster name is cluster_{module}')
#             cluster.attr(
#                 label=module.name,
#                 style='rounded,filled',
#                 color='bisque',
#                 # rankdir='LR',
#                 rank='same',
#             )
#             cluster.node_attr.update(
#                 style='filled',
#                 color='white',
#                 rank='source',
#                 group=module.name,
#             )
#             descendants = graph.find_descendants(module)
#             selected_modules.update(descendants)
#             cluster.node(module.name)
#             for descendant in descendants:
#                 cluster.node(descendant.name)
#     # Hacky way of adding edge between clusters.
#     g.edge('grimp.application', 'grimp.domain',
#            ltail='cluster_grimp.application', lhead='cluster_grimp.domain')
#     g.edge('grimp.adaptors', 'grimp.domain',
#            ltail='cluster_grimp.adaptors', lhead='cluster_grimp.domain')
#
#     for module in selected_modules:
#         for imported_module in graph.find_modules_directly_imported_by(module):
#             g.edge(
#                 module.name,
#                 imported_module.name,
#             )
#
#
#     # with g.subgraph(name='cluster_0') as c:
#     #     c.attr(style='filled')
#     #     c.attr(color='aliceblue')
#     #     c.node_attr.update(style='filled', color='white')
#     #     c.edges([('a0', 'a1'), ('a1', 'a2'), ('a2', 'a3')])
#     #     c.attr(label='foo.bar')
#     #
#     # g.edge('start', 'a0')
#     # g.edge('start', 'b0')
#     # g.edge('a1', 'b3')
#     # g.edge('b2', 'a3')
#     # g.edge('a3', 'a0')
#     # g.edge('a3', 'end')
#     # g.edge('b3', 'end')
#
#     g.view()

# def transform_graph(
#     graph: AbstractImportGraph,
#     new_root_module : Optional[Module] = None,
#     internal_depth_limit: Optional[int] = None,
#     external_depth_limit: Optional[int] = 1,
# ) -> AbstractImportGraph:
#     new_graph: AbstractImportGraph = settings.IMPORT_GRAPH_CLASS()
#
#     return new_graph