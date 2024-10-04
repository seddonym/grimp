from __future__ import annotations

from typing import Optional

from . import graph as python_graph
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

class ImportGraph(python_graph.ImportGraph):
    """
    Rust-backed implementation of the ImportGraph.
    """
    def __init__(self) -> None:
        super().__init__()
        self._rustgraph = rust.Graph()

    def add_import(
        self,
        *,
        importer: str,
        imported: str,
        line_number: Optional[int] = None,
        line_contents: Optional[str] = None,
    ) -> None:
        self._rustgraph.add_import(
            importer,
            imported,
            line_number,
            line_contents,
        )
        super().add_import(importer,imported,line_number,line_contents,)