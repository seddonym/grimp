use std::collections::hash_map::Entry::Vacant;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone)]
pub struct ImportGraph<'a> {
    pub names_by_id: HashMap<u32, &'a str>,
    pub ids_by_name: HashMap<&'a str, u32>,
    pub importers_by_imported: HashMap<u32, HashSet<u32>>,
    pub importeds_by_importer: HashMap<u32, HashSet<u32>>,
}

impl<'a> ImportGraph<'a> {
    pub fn new(importeds_by_importer: HashMap<&'a str, HashSet<&'a str>>) -> ImportGraph<'a> {
        // Build the name/id lookup maps.
        let mut names_by_id: HashMap<u32, &'a str> = HashMap::new();
        let mut ids_by_name: HashMap<&'a str, u32> = HashMap::new();
        let mut current_id: u32 = 1;
        for name in importeds_by_importer.keys() {
            names_by_id.insert(current_id, name);
            ids_by_name.insert(name, current_id);
            current_id += 1;
        }

        // Convert importeds_by_importer to id-based.
        let mut importeds_by_importer_u32: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (importer_str, importeds_strs) in importeds_by_importer.iter() {
            let mut importeds_u32 = HashSet::new();
            for imported_str in importeds_strs {
                importeds_u32.insert(*ids_by_name.get(imported_str).unwrap());
            }

            importeds_by_importer_u32
                .insert(*ids_by_name.get(importer_str).unwrap(), importeds_u32);
        }

        let importers_by_imported_u32 =
            ImportGraph::_build_importers_by_imported_u32(&importeds_by_importer_u32);

        ImportGraph {
            names_by_id,
            ids_by_name,
            importers_by_imported: importers_by_imported_u32,
            importeds_by_importer: importeds_by_importer_u32,
        }
    }

    fn _build_importers_by_imported_u32(
        importeds_by_importer_u32: &HashMap<u32, HashSet<u32>>,
    ) -> HashMap<u32, HashSet<u32>> {
        // Build importers_by_imported from importeds_by_importer.
        let mut importers_by_imported_u32: HashMap<u32, HashSet<u32>> = HashMap::new();
        for (importer, importeds) in importeds_by_importer_u32.iter() {
            for imported in importeds {
                let entry = importers_by_imported_u32.entry(*imported).or_default();
                entry.insert(*importer);
            }
        }

        // Check that there is an empty set for any remaining.
        for importer in importeds_by_importer_u32.keys() {
            importers_by_imported_u32.entry(*importer).or_default();
        }
        importers_by_imported_u32
    }

    pub fn get_module_ids(&self) -> HashSet<u32> {
        self.names_by_id.keys().copied().collect()
    }

    pub fn contains_module(&self, module_name: &str) -> bool {
        self.ids_by_name.contains_key(module_name)
    }

    pub fn remove_import(&mut self, importer: &str, imported: &str) {
        self.remove_import_ids(self.ids_by_name[importer], self.ids_by_name[imported]);
    }

    pub fn add_import_ids(&mut self, importer: u32, imported: u32) {
        let mut importeds = self.importeds_by_importer[&importer].clone();
        importeds.insert(imported);
        self.importeds_by_importer.insert(importer, importeds);

        let mut importers = self.importers_by_imported[&imported].clone();
        importers.insert(importer);
        self.importers_by_imported.insert(imported, importers);
    }

    pub fn remove_import_ids(&mut self, importer: u32, imported: u32) {
        let mut importeds = self.importeds_by_importer[&importer].clone();
        importeds.remove(&imported);
        self.importeds_by_importer.insert(importer, importeds);

        let mut importers = self.importers_by_imported[&imported].clone();
        importers.remove(&importer);
        self.importers_by_imported.insert(imported, importers);
    }

    pub fn remove_module_by_id(&mut self, module_id: u32) {
        let _module = self.names_by_id[&module_id];

        let mut imports_to_remove = Vec::with_capacity(self.names_by_id.len());
        {
            for imported_id in &self.importeds_by_importer[&module_id] {
                imports_to_remove.push((module_id, *imported_id));
            }
            for importer_id in &self.importers_by_imported[&module_id] {
                imports_to_remove.push((*importer_id, module_id));
            }
        }

        for (importer, imported) in imports_to_remove {
            self.remove_import_ids(importer, imported);
        }

        self.importeds_by_importer.remove(&module_id);
        self.importers_by_imported.remove(&module_id);
    }

    pub fn get_descendant_ids(&self, module_name: &str) -> Vec<u32> {
        let mut descendant_ids = vec![];
        for (candidate_name, candidate_id) in &self.ids_by_name {
            let namespace: String = format!("{}.", module_name);
            if candidate_name.starts_with(&namespace) {
                descendant_ids.push(*candidate_id);
            }
        }
        descendant_ids
    }

    pub fn remove_package(&mut self, module_name: &str) {
        for descendant_id in self.get_descendant_ids(module_name) {
            self.remove_module_by_id(descendant_id);
        }
        self.remove_module_by_id(self.ids_by_name[&module_name]);
    }

    pub fn squash_module(&mut self, module_name: &str) {
        let squashed_root_id = self.ids_by_name[module_name];
        let descendant_ids = &self.get_descendant_ids(module_name);

        // Assemble imports to add first, then add them in a second loop,
        // to avoid needing to clone importeds_by_importer.
        let mut imports_to_add = Vec::with_capacity(self.names_by_id.len());
        // Imports from the root.
        {
            for descendant_id in descendant_ids {
                for imported_id in &self.importeds_by_importer[&descendant_id] {
                    imports_to_add.push((squashed_root_id, *imported_id));
                }
                for importer_id in &self.importers_by_imported[&descendant_id] {
                    imports_to_add.push((*importer_id, squashed_root_id));
                }
            }
        }

        for (importer, imported) in imports_to_add {
            self.add_import_ids(importer, imported);
        }

        // Now we've added imports to/from the root, we can delete the root's descendants.
        for descendant_id in descendant_ids {
            self.remove_module_by_id(*descendant_id);
        }
    }

    pub fn pop_shortest_chains(&mut self, importer: &str, imported: &str) -> Vec<Vec<u32>> {
        let mut chains = vec![];
        let importer_id = self.ids_by_name[&importer];
        let imported_id = self.ids_by_name[&imported];

        while let Some(chain) = self.find_shortest_chain(importer_id, imported_id) {
            // Remove chain
            let _mods: Vec<&str> = chain.iter().map(|i| self.names_by_id[&i]).collect();
            for i in 0..chain.len() - 1 {
                self.remove_import_ids(chain[i], chain[i + 1]);
            }
            chains.push(chain);
        }

        chains
    }

    pub fn find_shortest_chain(&self, importer_id: u32, imported_id: u32) -> Option<Vec<u32>> {
        let results_or_none = self._search_for_path(importer_id, imported_id);
        match results_or_none {
            Some(results) => {
                let (pred, succ, initial_w) = results;

                let mut w_or_none: Option<u32> = Some(initial_w);
                // Transform results into vector.
                let mut path: Vec<u32> = Vec::new();
                // From importer to w:
                while w_or_none.is_some() {
                    let w = w_or_none.unwrap();
                    path.push(w);
                    w_or_none = pred[&w];
                }
                path.reverse();

                // From w to imported:
                w_or_none = succ[path.last().unwrap()];
                while w_or_none.is_some() {
                    let w = w_or_none.unwrap();
                    path.push(w);
                    w_or_none = succ[&w];
                }

                Some(path)
            }
            None => None,
        }
    }
    /// Performs a breadth first search from both source and target, meeting in the middle.
    //
    //  Returns:
    //      (pred, succ, w) where
    //         - pred is a dictionary of predecessors from w to the source, and
    //         - succ is a dictionary of successors from w to the target.
    //
    fn _search_for_path(
        &self,
        importer: u32,
        imported: u32,
    ) -> Option<(HashMap<u32, Option<u32>>, HashMap<u32, Option<u32>>, u32)> {
        if importer == imported {
            Some((
                HashMap::from([(imported, None)]),
                HashMap::from([(importer, None)]),
                importer,
            ))
        } else {
            let mut pred: HashMap<u32, Option<u32>> = HashMap::from([(importer, None)]);
            let mut succ: HashMap<u32, Option<u32>> = HashMap::from([(imported, None)]);

            // Initialize fringes, start with forward.
            let mut forward_fringe: Vec<u32> = Vec::from([importer]);
            let mut reverse_fringe: Vec<u32> = Vec::from([imported]);
            let mut this_level: Vec<u32>;

            while !forward_fringe.is_empty() && !reverse_fringe.is_empty() {
                if forward_fringe.len() <= reverse_fringe.len() {
                    this_level = forward_fringe.to_vec();
                    forward_fringe = Vec::new();
                    for v in this_level {
                        for w in self.importeds_by_importer[&v].clone() {
                            pred.entry(w).or_insert_with(|| {
                                forward_fringe.push(w);
                                Some(v)
                            });
                            if succ.contains_key(&w) {
                                // Found path.
                                return Some((pred, succ, w));
                            }
                        }
                    }
                } else {
                    this_level = reverse_fringe.to_vec();
                    reverse_fringe = Vec::new();
                    for v in this_level {
                        for w in self.importers_by_imported[&v].clone() {
                            if let Vacant(e) = succ.entry(w) {
                                e.insert(Some(v));
                                reverse_fringe.push(w);
                            }
                            if pred.contains_key(&w) {
                                // Found path.
                                return Some((pred, succ, w));
                            }
                        }
                    }
                }
            }
            None
        }
    }
}

impl fmt::Display for ImportGraph<'_> {
    fn fmt(&self, dest: &mut fmt::Formatter) -> fmt::Result {
        let mut strings = vec![];
        for (importer, importeds) in self.importeds_by_importer.iter() {
            let mut string = format!("IMPORTER {}: ", self.names_by_id[&importer]);
            for imported in importeds {
                string.push_str(format!("{}, ", self.names_by_id[&imported]).as_str());
            }
            strings.push(string);
        }
        strings.push("      ".to_string());
        for (imported, importers) in self.importers_by_imported.iter() {
            let mut string = format!("IMPORTED {}: ", self.names_by_id[&imported]);
            for importer in importers {
                string.push_str(format!("{}, ", self.names_by_id[&importer]).as_str());
            }
            strings.push(string);
        }
        write!(dest, "{}", strings.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _make_graph() -> ImportGraph<'static> {
        ImportGraph::new(HashMap::from([
            ("blue", HashSet::from(["blue.alpha", "blue.beta", "green"])),
            ("blue.alpha", HashSet::new()),
            ("blue.beta", HashSet::new()),
            ("green", HashSet::from(["blue.alpha", "blue.beta"])),
        ]))
    }

    #[test]
    fn get_module_ids() {
        let graph = _make_graph();

        assert_eq!(
            graph.get_module_ids(),
            HashSet::from([
                *graph.ids_by_name.get("blue").unwrap(),
                *graph.ids_by_name.get("blue.alpha").unwrap(),
                *graph.ids_by_name.get("blue.beta").unwrap(),
                *graph.ids_by_name.get("green").unwrap(),
            ])
        );
    }

    #[test]
    fn new_stores_importeds_by_importer_using_id() {
        let graph = _make_graph();

        let expected_importeds: HashSet<u32> = HashSet::from([
            *graph.ids_by_name.get("blue.alpha").unwrap(),
            *graph.ids_by_name.get("blue.beta").unwrap(),
            *graph.ids_by_name.get("green").unwrap(),
        ]);

        assert_eq!(
            *graph
                .importeds_by_importer
                .get(graph.ids_by_name.get("blue").unwrap())
                .unwrap(),
            expected_importeds
        );
    }

    #[test]
    fn new_stores_importers_by_imported_using_id() {
        let graph = _make_graph();

        let expected_importers: HashSet<u32> = HashSet::from([
            *graph.ids_by_name.get("blue").unwrap(),
            *graph.ids_by_name.get("green").unwrap(),
        ]);

        assert_eq!(
            *graph
                .importers_by_imported
                .get(graph.ids_by_name.get("blue.alpha").unwrap())
                .unwrap(),
            expected_importers
        );
    }

    #[test]
    fn test_squash_module() {
        let mut graph = ImportGraph::new(HashMap::from([
            ("blue", HashSet::from(["orange", "green"])),
            ("blue.alpha", HashSet::from(["green.delta"])),
            ("blue.beta", HashSet::new()),
            ("green", HashSet::from(["blue.alpha", "blue.beta"])),
            ("green.gamma", HashSet::new()),
            ("green.delta", HashSet::new()),
            ("orange", HashSet::new()),
        ]));

        graph.squash_module("blue");

        assert_eq!(
            graph.importeds_by_importer[&graph.ids_by_name["blue"]],
            HashSet::from([
                graph.ids_by_name["orange"],
                graph.ids_by_name["green"],
                graph.ids_by_name["green.delta"],
            ])
        );
        assert_eq!(
            graph.importeds_by_importer[&graph.ids_by_name["green"]],
            HashSet::from([graph.ids_by_name["blue"],])
        );

        assert_eq!(
            graph.importers_by_imported[&graph.ids_by_name["orange"]],
            HashSet::from([graph.ids_by_name["blue"],])
        );
        assert_eq!(
            graph.importers_by_imported[&graph.ids_by_name["green"]],
            HashSet::from([graph.ids_by_name["blue"],])
        );
        assert_eq!(
            graph.importers_by_imported[&graph.ids_by_name["green.delta"]],
            HashSet::from([graph.ids_by_name["blue"],])
        );
        assert_eq!(
            graph.importers_by_imported[&graph.ids_by_name["blue"]],
            HashSet::from([graph.ids_by_name["green"],])
        );
    }

    #[test]
    fn test_find_shortest_chain() {
        let blue = "blue";
        let green = "green";
        let yellow = "yellow";
        let blue_alpha = "blue.alpha";
        let blue_beta = "blue.beta";

        let graph = ImportGraph::new(HashMap::from([
            (green, HashSet::from([blue])),
            (blue_alpha, HashSet::from([blue])),
            (yellow, HashSet::from([green])),
            (blue_beta, HashSet::from([green])),
            (blue, HashSet::new()),
        ]));

        let path_or_none: Option<Vec<u32>> =
            graph.find_shortest_chain(graph.ids_by_name[&yellow], graph.ids_by_name[&blue]);

        assert_eq!(
            path_or_none,
            Some(Vec::from([
                graph.ids_by_name[&yellow],
                graph.ids_by_name[&green],
                graph.ids_by_name[&blue]
            ]))
        );
    }
}
