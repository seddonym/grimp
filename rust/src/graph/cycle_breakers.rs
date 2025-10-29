use crate::errors::GrimpResult;

use crate::graph::{Graph, ModuleToken};
use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::SecondaryMap;
use std::cmp::Ordering;

impl Graph {
    pub fn nominate_cycle_breakers(
        &self,
        package: ModuleToken,
    ) -> GrimpResult<FxHashSet<(ModuleToken, ModuleToken)>> {
        // Get children of this package.
        let children = match self.module_children.get(package) {
            Some(children) => children,
            None => return Ok(FxHashSet::default()),
        };
        if children.len() < 2 {
            return Ok(FxHashSet::default());
        }
        let (orig_imports, orig_reverse_imports, edge_weights) =
            self.build_graph_maps_of_children(children);

        // Make a copy of the graph. We'll iteratively remove nodes from this.
        let mut working_imports = orig_imports.clone();
        let mut working_reverse_imports = orig_reverse_imports.clone();

        // Iteratively extract sources into a vec.
        let mut sources: Vec<ModuleToken> = vec![];

        loop {
            let current_sources: Vec<ModuleToken> = working_reverse_imports
                .iter()
                .filter(|(_imported, importers)| importers.is_empty())
                .map(|(source, _)| source)
                .collect();

            if current_sources.is_empty() {
                break;
            }

            sources.extend(current_sources);
            // Remove sources from graph, then try again for another round.
            for source in sources.iter() {
                remove_module_from_graph(
                    source,
                    &mut working_imports,
                    &mut working_reverse_imports,
                );
            }
        }

        // Iteratively extract sinks into a vec.
        let mut sinks: Vec<ModuleToken> = vec![];
        loop {
            let new_sinks: Vec<ModuleToken> = working_imports
                .iter()
                .filter(|(_importer, importeds)| importeds.is_empty())
                .map(|(sink, _)| sink)
                .collect();

            if new_sinks.is_empty() {
                break;
            }

            // Remove sinks from graph, then try again for another round.
            for sink in new_sinks.iter() {
                remove_module_from_graph(sink, &mut working_imports, &mut working_reverse_imports);
            }
            // Add the new sinks to the beginning of sinks - these need to appear earlier in
            // the overall order, as they depend on the sinks previously found.
            sinks.splice(0..0, new_sinks);
            //sinks.extend(new_sinks);
        }

        // Iteratively extract remaining nodes, based on out-degree - in-degree.
        let mut middle: Vec<ModuleToken> = vec![];
        loop {
            if working_imports.is_empty() {
                // We've finished ordering the nodes.
                break;
            }

            // Initialize with the first node.
            let mut node_with_highest_difference: Option<ModuleToken> = None;
            let mut highest_difference_so_far: Option<isize> = None;

            for (candidate, _) in working_imports.iter() {
                let difference = calculate_degree_difference(
                    &candidate,
                    &working_imports,
                    &working_reverse_imports,
                    &edge_weights,
                );

                match node_with_highest_difference {
                    Some(incumbent) => {
                        match difference.cmp(&highest_difference_so_far.unwrap()) {
                            Ordering::Greater => {
                                highest_difference_so_far = Some(difference);
                                node_with_highest_difference = Some(candidate);
                            }
                            Ordering::Equal => {
                                // Tie breaker - choose the earlier one alphabetically.
                                let incumbent_name = self.get_module(incumbent).unwrap().name();
                                let candidate_name = self.get_module(candidate).unwrap().name();

                                if candidate_name < incumbent_name {
                                    highest_difference_so_far = Some(difference);
                                    node_with_highest_difference = Some(candidate);
                                }
                            }
                            Ordering::Less => {}
                        }
                    }
                    None => {
                        node_with_highest_difference = Some(candidate);
                        highest_difference_so_far = Some(difference);
                    }
                }
            }

            // Extract node with highest difference to one of the two middle vecs.
            let node_with_highest_difference = node_with_highest_difference.unwrap();

            // Prioritize high out-degree.
            middle.push(node_with_highest_difference);
            remove_module_from_graph(
                &node_with_highest_difference,
                &mut working_imports,
                &mut working_reverse_imports,
            );
        }
        if !working_reverse_imports.is_empty() {
            panic!("Expected reverse imports also to be empty.");
        }

        // Combine sources + ordered + sinks.
        let full_ordering: Vec<_> = sources.into_iter().chain(middle).chain(sinks).collect();
        // Iterate through all edges in original graph.
        // If the edge points leftwards in the full ordering, it is a circuit breaker.
        let mut package_cycle_breakers: Vec<(ModuleToken, ModuleToken)> = vec![];
        for (importer, importeds) in orig_imports.iter() {
            for imported in importeds.iter() {
                let importer_position = get_module_position(&full_ordering, &importer).unwrap();
                let imported_position = get_module_position(&full_ordering, imported).unwrap();
                if imported_position < importer_position {
                    package_cycle_breakers.push((importer, *imported));
                }
            }
        }

        // Expand the package cycle breakers to the specific imports from the unsquashed graph.
        let mut cycle_breakers = FxHashSet::default();
        for (importer, imported) in package_cycle_breakers {
            cycle_breakers.extend(
                self.find_direct_imports_between(importer, imported, true)
                    .unwrap(),
            );
        }

        Ok(cycle_breakers)
    }

    // These maps represent the local graph. Each module appears as a key in the maps,
    // with empty sets if they have no imports to / from.
    // They are denormalized into two maps for fast lookups.
    #[allow(clippy::type_complexity)]
    fn build_graph_maps_of_children(
        &self,
        children: &FxHashSet<ModuleToken>,
    ) -> (
        SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>, // Imports
        SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>, // Reverse imports
        FxHashMap<(ModuleToken, ModuleToken), usize>,      // Weights
    ) {
        let mut orig_imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>> =
            SecondaryMap::default();
        let mut orig_reverse_imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>> =
            SecondaryMap::default();
        let mut weights = FxHashMap::default();

        for child in children {
            orig_imports.insert(*child, FxHashSet::default());
            orig_reverse_imports.insert(*child, FxHashSet::default());
        }
        // Get number of imports between each child.
        // Add as edges.
        for child_a in children {
            for child_b in children.iter().filter(|child| *child != child_a) {
                // Get number of imports from child_a to child_b (as package).
                let num_imports_from_a_to_b = self
                    .find_direct_imports_between(*child_a, *child_b, true)
                    .unwrap()
                    .len();

                if num_imports_from_a_to_b > 0 {
                    // a depends on b - add it as an edge in our temporary graph.
                    orig_imports[*child_a].insert(*child_b);
                    orig_reverse_imports[*child_b].insert(*child_a);
                    weights.insert((*child_a, *child_b), num_imports_from_a_to_b);
                }
            }
        }
        (orig_imports, orig_reverse_imports, weights)
    }
}

/// Removes the module from the graph, along with any imports to or from it.
fn remove_module_from_graph(
    module: &ModuleToken,
    imports: &mut SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    reverse_imports: &mut SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
) {
    imports.remove(*module);
    for (_, importeds) in imports.iter_mut() {
        importeds.remove(module);
    }
    reverse_imports.remove(*module);
    for (_, importers) in reverse_imports.iter_mut() {
        importers.remove(module);
    }
}

/// Removes the module from the graph, along with any imports to or from it.
fn calculate_degree_difference(
    module: &ModuleToken,
    imports: &SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    reverse_imports: &SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    edge_weights: &FxHashMap<(ModuleToken, ModuleToken), usize>,
) -> isize {
    let importer_modules = &reverse_imports[*module];
    let imported_modules = &imports[*module];

    let indegree: isize = importer_modules
        .iter()
        .map(|importer| edge_weights[&(*importer, *module)] as isize)
        .sum();
    let outdegree: isize = imported_modules
        .iter()
        .map(|imported| edge_weights[&(*module, *imported)] as isize)
        .sum();

    outdegree - indegree
}

fn get_module_position(vec: &[ModuleToken], module: &ModuleToken) -> Option<usize> {
    vec.iter().position(|&i| i == *module)
}
