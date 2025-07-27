use crate::errors::{GrimpError, GrimpResult};
use crate::graph::{EMPTY_MODULE_TOKENS, Graph, ModuleToken};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use slotmap::SecondaryMap;
use std::hash::BuildHasherDefault;

type FxIndexSet<K> = IndexSet<K, BuildHasherDefault<FxHasher>>;
type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

pub fn find_reach(
    imports_map: &SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    from_modules: &FxHashSet<ModuleToken>,
) -> FxHashSet<ModuleToken> {
    let mut seen = FxIndexSet::default();
    seen.extend(from_modules.iter().cloned());

    let mut i = 0;
    while let Some(module) = seen.get_index(i) {
        for next_module in imports_map.get(*module).unwrap_or(&EMPTY_MODULE_TOKENS) {
            if !seen.contains(next_module) {
                seen.insert(*next_module);
            }
        }
        i += 1;
    }

    &seen.into_iter().collect::<FxHashSet<_>>() - from_modules
}

/// Finds the shortest path, via a bidirectional BFS.
pub fn find_shortest_path(
    graph: &Graph,
    from_modules: &FxHashSet<ModuleToken>,
    to_modules: &FxHashSet<ModuleToken>,
    excluded_modules: &FxHashSet<ModuleToken>,
    excluded_imports: &FxHashMap<ModuleToken, FxHashSet<ModuleToken>>,
) -> GrimpResult<Option<Vec<ModuleToken>>> {
    if !(from_modules & to_modules).is_empty() {
        return Err(GrimpError::SharedDescendants);
    }

    let predecessors: FxIndexMap<ModuleToken, Option<ModuleToken>> = from_modules
        .clone()
        .into_iter()
        .map(|m| (m, None))
        .collect();
    let successors: FxIndexMap<ModuleToken, Option<ModuleToken>> =
        to_modules.clone().into_iter().map(|m| (m, None)).collect();

    _find_shortest_path(
        graph,
        predecessors,
        successors,
        excluded_modules,
        excluded_imports,
    )
}

/// Finds the shortest cycle from `modules` to `modules`, via a bidirectional BFS.
pub fn find_shortest_cycle(
    graph: &Graph,
    modules: &[ModuleToken],
    excluded_modules: &FxHashSet<ModuleToken>,
    excluded_imports: &FxHashMap<ModuleToken, FxHashSet<ModuleToken>>,
) -> GrimpResult<Option<Vec<ModuleToken>>> {
    // Exclude imports internal to `modules`
    let mut excluded_imports = excluded_imports.clone();
    for (m1, m2) in modules.iter().tuple_combinations() {
        excluded_imports.entry(*m1).or_default().insert(*m2);
        excluded_imports.entry(*m2).or_default().insert(*m1);
    }

    let predecessors: FxIndexMap<ModuleToken, Option<ModuleToken>> = modules
        .iter()
        .cloned()
        .map(|m| (m, None))
        .collect();

    let successors: FxIndexMap<ModuleToken, Option<ModuleToken>> = predecessors
        .clone();

    _find_shortest_path(graph, predecessors, successors, excluded_modules, &excluded_imports)
}

fn _find_shortest_path(
    graph: &Graph,
    mut predecessors: FxIndexMap<ModuleToken, Option<ModuleToken>>,
    mut successors: FxIndexMap<ModuleToken, Option<ModuleToken>>,
    excluded_modules: &FxHashSet<ModuleToken>,
    excluded_imports: &FxHashMap<ModuleToken, FxHashSet<ModuleToken>>,
) -> GrimpResult<Option<Vec<ModuleToken>>> {
    

    let mut i_forwards = 0;
    let mut i_backwards = 0;
    let middle = 'l: loop {
        for _ in 0..(predecessors.len() - i_forwards) {
            let module = *predecessors.get_index(i_forwards).unwrap().0;
            let mut next_modules: Vec<_> = graph.imports.get(module).unwrap().iter().cloned().collect();
            next_modules.sort_by_key(|next_module| graph.get_module(*next_module).unwrap().name());
            for next_module in next_modules {
                if import_is_excluded(&module, &next_module, excluded_modules, excluded_imports) {
                    continue;
                }
                if !predecessors.contains_key(&next_module) {
                    predecessors.insert(next_module, Some(module));
                }
                if successors.contains_key(&next_module) {
                    break 'l Some(next_module);
                }
            }
            i_forwards += 1;
        }

        for _ in 0..(successors.len() - i_backwards) {
            let module = *successors.get_index(i_backwards).unwrap().0;
            let mut next_modules: Vec<_> = graph.reverse_imports.get(module).unwrap().iter().cloned().collect();
            next_modules.sort_by_key(|next_module| graph.get_module(*next_module).unwrap().name());
            for next_module in next_modules {
                if import_is_excluded(&next_module, &module, excluded_modules, excluded_imports) {
                    continue;
                }
                if !successors.contains_key(&next_module) {
                    successors.insert(next_module, Some(module));
                }
                if predecessors.contains_key(&next_module) {
                    break 'l Some(next_module);
                }
            }
            i_backwards += 1;
        }

        if i_forwards == predecessors.len() && i_backwards == successors.len() {
            break 'l None;
        }
    };

    Ok(middle.map(|middle| {
        // Path found!
        // Build the path.
        let mut path = vec![];
        let mut node = Some(middle);
        while let Some(n) = node {
            path.push(n);
            node = *predecessors.get(&n).unwrap();
        }
        path.reverse();
        let mut node = *successors.get(path.last().unwrap()).unwrap();
        while let Some(n) = node {
            path.push(n);
            node = *successors.get(&n).unwrap();
        }
        path
    }))
}

fn import_is_excluded(
    from_module: &ModuleToken,
    to_module: &ModuleToken,
    excluded_modules: &FxHashSet<ModuleToken>,
    excluded_imports: &FxHashMap<ModuleToken, FxHashSet<ModuleToken>>,
) -> bool {
    if excluded_modules.contains(to_module) {
        true
    } else {
        excluded_imports
            .get(from_module)
            .unwrap_or(&EMPTY_MODULE_TOKENS)
            .contains(to_module)
    }
}

#[cfg(test)]
mod test_find_shortest_cycle {
    use super::*;

    #[test]
    fn test_finds_cycle_single_module() -> GrimpResult<()> {
        let mut graph = Graph::default();
        let foo = graph.get_or_add_module("foo").token;
        let bar = graph.get_or_add_module("bar").token;
        let baz = graph.get_or_add_module("baz").token;
        let x = graph.get_or_add_module("x").token;
        let y = graph.get_or_add_module("y").token;
        let z = graph.get_or_add_module("z").token;
        // Shortest cycle
        graph.add_import(foo, bar);
        graph.add_import(bar, baz);
        graph.add_import(baz, foo);
        // Longer cycle
        graph.add_import(foo, x);
        graph.add_import(x, y);
        graph.add_import(y, z);
        graph.add_import(z, foo);

        let path = find_shortest_cycle(
            &graph,
            &[foo],
            &FxHashSet::default(),
            &FxHashMap::default(),
        )?;
        assert_eq!(path, Some(vec![foo, bar, baz, foo]));

        graph.remove_import(baz, foo);

        let path = find_shortest_cycle(
            &graph,
            &[foo],
            &FxHashSet::default(),
            &FxHashMap::default(),
        )?;
        assert_eq!(path, Some(vec![foo, x, y, z, foo]));

        Ok(())
    }

    #[test]
    fn test_returns_none_if_no_cycle() -> GrimpResult<()> {
        let mut graph = Graph::default();
        let foo = graph.get_or_add_module("foo").token;
        let bar = graph.get_or_add_module("bar").token;
        let baz = graph.get_or_add_module("baz").token;
        graph.add_import(foo, bar);
        graph.add_import(bar, baz);

        let path = find_shortest_cycle(
            &graph,
            &[foo],
            &FxHashSet::default(),
            &FxHashMap::default(),
        )?;

        assert_eq!(path, None);

        Ok(())
    }

    #[test]
    fn test_finds_cycle_multiple_module() -> GrimpResult<()> {
        let mut graph = Graph::default();

        graph.get_or_add_module("colors");
        let red = graph.get_or_add_module("colors.red").token;
        let blue = graph.get_or_add_module("colors.blue").token;
        let a = graph.get_or_add_module("a").token;
        let b = graph.get_or_add_module("b").token;
        let c = graph.get_or_add_module("c").token;
        let d = graph.get_or_add_module("d").token;

        // The computation should not be confused by these two imports internal to `modules`.
        graph.add_import(red, blue);
        graph.add_import(blue, red);
        // This is the part we expect to find.
        graph.add_import(red, a);
        graph.add_import(a, b);
        graph.add_import(b, blue);
        // A longer path.
        graph.add_import(a, c);
        graph.add_import(c, d);
        graph.add_import(d, b);

        let path = find_shortest_cycle(
            &graph,
            &Vec::from_iter([red, blue]),
            &FxHashSet::default(),
            &FxHashMap::default(),
        )?;

        assert_eq!(path, Some(vec![red, a, b, blue]));

        Ok(())
    }
}
