"""
The algorithms in this module have been adapted from networkx 2.8.5.

See networkx.algorithms.shortest_paths.unweighted.bidirectional_shortest_path.

Original license follows:

--------------------------------------------------------------------------

NetworkX is distributed with the 3-clause BSD license.

::

   Copyright (C) 2004-2022, NetworkX Developers
   Aric Hagberg <hagberg@lanl.gov>
   Dan Schult <dschult@colgate.edu>
   Pieter Swart <swart@lanl.gov>
   All rights reserved.

   Redistribution and use in source and binary forms, with or without
   modification, are permitted provided that the following conditions are
   met:

     * Redistributions of source code must retain the above copyright
       notice, this list of conditions and the following disclaimer.

     * Redistributions in binary form must reproduce the above
       copyright notice, this list of conditions and the following
       disclaimer in the documentation and/or other materials provided
       with the distribution.

     * Neither the name of the NetworkX Developers nor the names of its
       contributors may be used to endorse or promote products derived
       from this software without specific prior written permission.

   THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
   "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
   LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
   A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
   OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
   SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
   LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
   DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
   THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
   (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
   OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
"""
from typing import Dict, Optional, Set, Tuple


def bidirectional_shortest_path(
    *,
    importer: str,
    imported: str,
    importers_by_imported: Dict[str, Set[str]],
    importeds_by_importer: Dict[str, Set[str]],
) -> Optional[Tuple[str, ...]]:
    """
    Returns a tuple of modules in the shortest path between importer and imported.

    If no path can be found, return None.

    Args:
        importer: the module doing the importing; the starting point.
        imported: the module being imported: the end point.
        importers_by_imported: Map of modules directly imported by each key.
        importeds_by_importer: Map of all the modules that directly import each key.
    """

    results = _search_for_path(
        importers_by_imported=importers_by_imported,
        importeds_by_importer=importeds_by_importer,
        importer=importer,
        imported=imported,
    )
    if results is None:
        return None

    pred, succ, w = results

    # Transform results into tuple.
    path = []
    # From importer to w:
    while w is not None:
        path.append(w)
        w = pred[w]  # type: ignore
    path.reverse()
    # From w to imported:
    w = succ[path[-1]]
    while w is not None:
        path.append(w)
        w = succ[w]

    return tuple(path)


def _search_for_path(
    *, importers_by_imported, importeds_by_importer, importer: str, imported: str
) -> Optional[Tuple[Dict[str, Optional[str]], Dict[str, Optional[str]], str]]:
    """Bidirectional shortest path helper.

    Performs a breadth first search from both source and target, meeting in the middle.

    Returns:
         (pred, succ, w) where
            - pred is a dictionary of predecessors from w to the source, and
            - succ is a dictionary of successors from w to the target.
    """
    if imported == importer:
        return ({imported: None}, {importer: None}, importer)

    pred: Dict[str, Optional[str]] = {importer: None}
    succ: Dict[str, Optional[str]] = {imported: None}

    # Initialize fringes, start with forward.
    forward_fringe = [importer]
    reverse_fringe = [imported]

    while forward_fringe and reverse_fringe:
        if len(forward_fringe) <= len(reverse_fringe):
            this_level = forward_fringe
            forward_fringe = []
            for v in this_level:
                for w in importeds_by_importer[v]:
                    if w not in pred:
                        forward_fringe.append(w)
                        pred[w] = v
                    if w in succ:
                        # Found path.
                        return pred, succ, w
        else:
            this_level = reverse_fringe
            reverse_fringe = []
            for v in this_level:
                for w in importers_by_imported[v]:
                    if w not in succ:
                        succ[w] = v
                        reverse_fringe.append(w)
                    if w in pred:
                        # Found path.
                        return pred, succ, w

    return None
