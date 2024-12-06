/*
modules (get_modules) DONE
find_children - DONE
find_descendants - DONE
direct_import_exists - DONE
find_modules_directly_imported_by - DONE
find_modules_that_directly_import -  DONE
get_import_details - DONE
count_imports - DONE
find_upstream_modules - DONE
find_downstream_modules - DONE
find_shortest_chain - DONE
    find_shortest_chains - TODO
chain_exists - DONE
    find_illegal_dependencies_for_layers - TODO
add_module - DONE - need to add is_squashed
remove_module - DONE
add_import - DONE
remove_import - DONE
squash_module - DONE
is_module_squashed - DONE

Also, sensible behaviour when passing modules that don't exist in the graph.
*/
#![allow(dead_code)]

use bimap::BiMap;
use log::info;
use petgraph::algo::astar;
use petgraph::graph::EdgeIndex;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::{Bfs, Walker};
use petgraph::Direction;
use std::collections::{HashMap, HashSet};
use std::fmt;
//use std::time::Instant;

use crate::layers::Level;

// Delimiter for Python modules.
const DELIMITER: char = '.';

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Module {
    pub name: String,
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleNotPresent {
    pub module: Module,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoSuchContainer {
    pub container: String,
}

impl fmt::Display for ModuleNotPresent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\" not present in the graph", self.module.name)
    }
}

impl Module {
    pub fn new(name: String) -> Module {
        Module { name }
    }

    // Returns whether the module is a root-level package.
    pub fn is_root(&self) -> bool {
        !self.name.contains(DELIMITER)
    }

    // Create a Module that is the parent of the passed Module.
    //
    // Panics if the child is a root Module.
    pub fn new_parent(child: &Module) -> Module {
        let parent_name = match child.name.rsplit_once(DELIMITER) {
            Some((base, _)) => base.to_string(),
            None => panic!("{} is a root level package", child.name),
        };

        Module::new(parent_name)
    }

    // Return whether this module is a descendant of the supplied one, based on the name.
    pub fn is_descendant_of(&self, module: &Module) -> bool {
        let candidate = format!("{}{}", module.name, DELIMITER);
        self.name.starts_with(&candidate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DetailedImport {
    pub importer: Module,
    pub imported: Module,
    pub line_number: usize,
    pub line_contents: String,
}

#[derive(Default, Clone)]
pub struct Graph {
    // Bidirectional lookup between Module and NodeIndex.
    hierarchy_module_indices: BiMap<Module, NodeIndex>,
    hierarchy: StableGraph<Module, ()>,
    imports_module_indices: BiMap<Module, NodeIndex>,
    imports: StableGraph<Module, ()>,
    squashed_modules: HashSet<Module>,
    // Invisible modules exist in the hierarchy but haven't been explicitly added to the graph.
    invisible_modules: HashSet<Module>,
    detailed_imports_map: HashMap<(Module, Module), HashSet<DetailedImport>>,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Route {
    pub heads: Vec<Module>,
    pub middle: Vec<Module>,
    pub tails: Vec<Module>,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct PackageDependency {
    pub importer: Module,
    pub imported: Module,
    pub routes: Vec<Route>,
}

impl Graph {
    pub fn pretty_str(&self) -> String {
        let mut hierarchy: Vec<String> = vec![];
        let mut imports: Vec<String> = vec![];

        let hierarchy_module_indices: Vec<_> = self.hierarchy_module_indices.iter().collect();

        for (parent_module, parent_index) in hierarchy_module_indices {
            for child_index in self.hierarchy.neighbors(*parent_index) {
                let child_module = self
                    .hierarchy_module_indices
                    .get_by_right(&child_index)
                    .unwrap();
                let parent_module_str = match self.invisible_modules.contains(&parent_module) {
                    true => format!("({})", parent_module.name),
                    false => parent_module.name.to_string(),
                };
                let child_module_str = match self.invisible_modules.contains(&child_module) {
                    true => format!("({})", child_module.name),
                    false => child_module.name.to_string(),
                };
                hierarchy.push(format!("    {} -> {}", parent_module_str, child_module_str));
            }
        }

        let imports_module_indices: Vec<_> = self.imports_module_indices.iter().collect();

        for (from_module, from_index) in imports_module_indices {
            for to_index in self.imports.neighbors(*from_index) {
                let to_module = self.imports_module_indices.get_by_right(&to_index).unwrap();
                imports.push(format!("    {} -> {}", from_module.name, to_module.name));
            }
        }
        // Assemble String.
        let mut pretty = String::new();
        pretty.push_str("hierarchy:\n");
        hierarchy.sort();
        pretty.push_str(&hierarchy.join("\n"));
        pretty.push_str("\nimports:\n");
        imports.sort();
        pretty.push_str(&imports.join("\n"));
        pretty.push('\n');
        pretty
    }

    pub fn add_module(&mut self, module: Module) {
        // If this module is already in the graph, but invisible, just make it visible.
        if self.invisible_modules.contains(&module) {
            self.invisible_modules.remove(&module);
            return;
        }

        let module_index = self.hierarchy.add_node(module.clone());
        self.hierarchy_module_indices
            .insert(module.clone(), module_index);

        // Add this module to the hierarchy.
        if !module.is_root() {
            let parent = Module::new_parent(&module);

            let parent_index = match self.hierarchy_module_indices.get_by_left(&parent) {
                Some(index) => index,
                None => {
                    // If the parent isn't already in the graph, add it, but as an invisible module.
                    self.add_module(parent.clone());
                    self.invisible_modules.insert(parent.clone());
                    self.hierarchy_module_indices.get_by_left(&parent).unwrap()
                }
            };
            self.hierarchy.add_edge(*parent_index, module_index, ());
        }
    }

    pub fn add_squashed_module(&mut self, module: Module) {
        self.add_module(module.clone());
        self.squashed_modules.insert(module);
    }

    pub fn remove_module(&mut self, module: &Module) {
        // Remove imports by module.
        let imported_modules: Vec<Module> = self
            .find_modules_directly_imported_by(module)
            .iter()
            .map(|m| (*m).clone())
            .collect();
        for imported_module in imported_modules {
            self.remove_import(&module, &imported_module);
        }

        // Remove imports of module.
        let importer_modules: Vec<Module> = self
            .find_modules_that_directly_import(module)
            .iter()
            .map(|m| (*m).clone())
            .collect();
        for importer_module in importer_modules {
            self.remove_import(&importer_module, &module);
        }

        // Remove module from hierarchy.
        if let Some(hierarchy_index) = self.hierarchy_module_indices.get_by_left(module) {
            // TODO should we check for children before removing?
            // Maybe should just make invisible instead?
            self.hierarchy.remove_node(*hierarchy_index);
            self.hierarchy_module_indices.remove_by_left(module);
        };
    }

    pub fn get_modules(&self) -> HashSet<&Module> {
        self.hierarchy_module_indices
            .left_values()
            .filter(|module| !self.invisible_modules.contains(module))
            .collect()
    }

    pub fn count_imports(&self) -> usize {
        self.imports.edge_count()
    }

    pub fn get_import_details(
        &self,
        importer: &Module,
        imported: &Module,
    ) -> HashSet<DetailedImport> {
        let key = (importer.clone(), imported.clone());
        match self.detailed_imports_map.get(&key) {
            Some(import_details) => import_details.clone(),
            None => HashSet::new(),
        }
    }

    pub fn find_children(&self, module: &Module) -> HashSet<&Module> {
        if self.invisible_modules.contains(module) {
            return HashSet::new();
        }
        let module_index = match self.hierarchy_module_indices.get_by_left(module) {
            Some(index) => index,
            // Module does not exist.
            // TODO: should this return a result, to handle if module is not in graph?
            None => return HashSet::new(),
        };
        self.hierarchy
            .neighbors(*module_index)
            .map(|index| self.hierarchy_module_indices.get_by_right(&index).unwrap())
            .filter(|module| !self.invisible_modules.contains(module))
            .collect()
    }

    pub fn find_descendants(&self, module: &Module) -> Result<HashSet<&Module>, ModuleNotPresent> {
        let module_index = match self.hierarchy_module_indices.get_by_left(module) {
            Some(index) => index,
            None => {
                return Err(ModuleNotPresent {
                    module: module.clone(),
                })
            }
        };
        Ok(Bfs::new(&self.hierarchy, *module_index)
            .iter(&self.hierarchy)
            .filter(|index| index != module_index) // Don't include the supplied module.
            .map(|index| self.hierarchy_module_indices.get_by_right(&index).unwrap()) // This panics sometimes.
            .filter(|module| !self.invisible_modules.contains(module))
            .collect())
    }

    pub fn add_import(&mut self, importer: &Module, imported: &Module) {
        // Don't bother doing anything if it's already in the graph.
        if self.direct_import_exists(&importer, &imported, false) {
            return;
        }

        self.add_module_if_not_in_hierarchy(importer);
        self.add_module_if_not_in_hierarchy(imported);

        let importer_index: NodeIndex = match self.imports_module_indices.get_by_left(importer) {
            Some(index) => *index,
            None => {
                let index = self.imports.add_node(importer.clone());
                self.imports_module_indices.insert(importer.clone(), index);
                index
            }
        };
        let imported_index: NodeIndex = match self.imports_module_indices.get_by_left(imported) {
            Some(index) => *index,
            None => {
                let index = self.imports.add_node(imported.clone());
                self.imports_module_indices.insert(imported.clone(), index);
                index
            }
        };

        self.imports.add_edge(importer_index, imported_index, ());
        // println!(
        //     "Added {:?} {:?} -> {:?} {:?}, edge count now {:?}",
        //     importer,
        //     importer_index,
        //     imported,
        //     imported_index,
        //     self.imports.edge_count()
        // );
    }

    pub fn add_detailed_import(&mut self, import: &DetailedImport) {
        let key = (import.importer.clone(), import.imported.clone());
        self.detailed_imports_map
            .entry(key)
            .or_insert_with(HashSet::new)
            .insert(import.clone());
        self.add_import(&import.importer, &import.imported);
    }

    pub fn remove_import(&mut self, importer: &Module, imported: &Module) {
        let importer_index: NodeIndex = match self.imports_module_indices.get_by_left(importer) {
            Some(index) => *index,
            None => return,
        };
        let imported_index: NodeIndex = match self.imports_module_indices.get_by_left(imported) {
            Some(index) => *index,
            None => return,
        };
        let edge_index: EdgeIndex = match self.imports.find_edge(importer_index, imported_index) {
            Some(index) => index,
            None => return,
        };

        self.imports.remove_edge(edge_index);

        // There might be other imports to / from the modules, so don't
        // remove from the indices.  (TODO: does it matter if we don't clean these up
        // if there are no more imports?)
        // self.imports_module_indices.remove_by_left(importer);
        // self.imports_module_indices.remove_by_left(importer);

        let key = (importer.clone(), imported.clone());

        self.detailed_imports_map.remove(&key);
        self.imports.remove_edge(edge_index);
    }

    // Note: this will panic if importer and imported are in the same package.
    #[allow(unused_variables)]
    pub fn direct_import_exists(
        &self,
        importer: &Module,
        imported: &Module,
        as_packages: bool,
    ) -> bool {
        let graph_to_use: &Graph;
        let mut graph_copy: Graph;

        if as_packages {
            graph_copy = self.clone();
            graph_copy.squash_module(importer);
            graph_copy.squash_module(imported);
            graph_to_use = &graph_copy;
        } else {
            graph_to_use = self;
        }

        // The modules may appear in the hierarchy, but have no imports, so we
        // return false unless they're both in there.
        let importer_index = match graph_to_use.imports_module_indices.get_by_left(importer) {
            Some(importer_index) => *importer_index,
            None => return false,
        };
        let imported_index = match graph_to_use.imports_module_indices.get_by_left(imported) {
            Some(imported_index) => *imported_index,
            None => return false,
        };

        graph_to_use
            .imports
            .contains_edge(importer_index, imported_index)
    }

    pub fn find_modules_that_directly_import(&self, imported: &Module) -> HashSet<&Module> {
        let imported_index = match self.imports_module_indices.get_by_left(imported) {
            Some(imported_index) => *imported_index,
            None => return HashSet::new(),
        };
        println!(
            "module, {:?}, imported_index {:?}",
            imported, imported_index
        );
        let importer_indices: HashSet<NodeIndex> = self
            .imports
            .neighbors_directed(imported_index, Direction::Incoming)
            .collect();

        println!("importer indices {:?}", importer_indices);
        for i in importer_indices.iter() {
            println!(
                "Importer {:?}",
                self.imports_module_indices.get_by_right(i).unwrap()
            );
        }
        let importers: HashSet<&Module> = importer_indices
            .iter()
            .map(|importer_index| {
                self.imports_module_indices
                    .get_by_right(importer_index)
                    .unwrap()
            })
            .collect();
        importers
    }

    pub fn find_modules_directly_imported_by(&self, importer: &Module) -> HashSet<&Module> {
        let importer_index = match self.imports_module_indices.get_by_left(importer) {
            Some(importer_index) => *importer_index,
            None => return HashSet::new(),
        };
        let imported_indices: HashSet<NodeIndex> = self
            .imports
            .neighbors_directed(importer_index, Direction::Outgoing)
            .collect();

        let importeds: HashSet<&Module> = imported_indices
            .iter()
            .map(|imported_index| {
                self.imports_module_indices
                    .get_by_right(imported_index)
                    .unwrap()
            })
            .collect();
        importeds
    }

    pub fn find_upstream_modules(&self, module: &Module, as_package: bool) -> HashSet<&Module> {
        let mut upstream_modules = HashSet::new();

        let mut modules_to_check: HashSet<&Module> = HashSet::from([module]);
        if as_package {
            let descendants = self.find_descendants(&module).unwrap_or(HashSet::new());
            modules_to_check.extend(descendants.into_iter());
        };

        for module_to_check in modules_to_check.iter() {
            let module_index = match self.imports_module_indices.get_by_left(module_to_check) {
                Some(index) => *index,
                None => continue,
            };
            upstream_modules.extend(
                Bfs::new(&self.imports, module_index)
                    .iter(&self.imports)
                    .map(|index| self.imports_module_indices.get_by_right(&index).unwrap())
                    // Exclude any modules that we are checking.
                    .filter(|downstream_module| !modules_to_check.contains(downstream_module)),
            );
        }

        upstream_modules
    }

    pub fn find_downstream_modules(&self, module: &Module, as_package: bool) -> HashSet<&Module> {
        let mut downstream_modules = HashSet::new();

        let mut modules_to_check: HashSet<&Module> = HashSet::from([module]);
        if as_package {
            let descendants = self.find_descendants(&module).unwrap_or(HashSet::new());
            modules_to_check.extend(descendants.into_iter());
        };

        for module_to_check in modules_to_check.iter() {
            let module_index = match self.imports_module_indices.get_by_left(module_to_check) {
                Some(index) => *index,
                None => continue,
            };

            // Reverse all the edges in the graph and then do what we do in find_upstream_modules.
            // Is there a way of doing this without the clone?
            let mut reversed_graph = self.imports.clone();
            reversed_graph.reverse();

            downstream_modules.extend(
                Bfs::new(&reversed_graph, module_index)
                    .iter(&reversed_graph)
                    .map(|index| self.imports_module_indices.get_by_right(&index).unwrap())
                    // Exclude any modules that we are checking.
                    .filter(|downstream_module| !modules_to_check.contains(downstream_module)),
            )
        }

        downstream_modules
    }

    pub fn find_shortest_chain(
        &self,
        importer: &Module,
        imported: &Module,
    ) -> Option<Vec<&Module>> {
        let importer_index = match self.imports_module_indices.get_by_left(importer) {
            Some(index) => *index,
            None => return None, // Importer has no imports to or from.
        };
        let imported_index = match self.imports_module_indices.get_by_left(imported) {
            Some(index) => *index,
            None => return None, // Imported has no imports to or from.
        };
        let path_to_imported = match astar(
            &self.imports,
            importer_index,
            |finish| finish == imported_index,
            |_e| 1,
            |_| 0,
        ) {
            Some(path_tuple) => path_tuple.1,
            None => return None, // No chain to the imported.
        };

        let mut chain: Vec<&Module> = vec![];
        for link_index in path_to_imported {
            let module = self
                .imports_module_indices
                .get_by_right(&link_index)
                .unwrap();
            chain.push(module);
        }
        Some(chain)
    }

    // https://github.com/seddonym/grimp/blob/2b37bd9268655f99439f376625e08151a075a5bd/src/grimp/adaptors/graph.py#L290
    pub fn find_shortest_chains(
        &self,
        importer: &Module,
        imported: &Module,
    ) -> HashSet<Vec<&Module>> {
        let mut chains = HashSet::new();

        let mut importer_modules: HashSet<&Module> = HashSet::from([importer]);
        // TODO don't do this if module is squashed?
        for descendant in self.find_descendants(&importer).unwrap() {
            importer_modules.insert(descendant);
        }

        let mut imported_modules: HashSet<&Module> = HashSet::from([imported]);
        // TODO don't do this if module is squashed?
        for descendant in self.find_descendants(&imported).unwrap() {
            imported_modules.insert(descendant);
        }

        // TODO - Error if modules have shared descendants.

        for importer_module in importer_modules {
            for imported_module in &imported_modules {
                if let Some(chain) = self.find_shortest_chain(importer_module, imported_module) {
                    chains.insert(chain);
                }
            }
        }
        chains
    }

    #[allow(unused_variables)]
    pub fn chain_exists(&self, importer: &Module, imported: &Module, as_packages: bool) -> bool {
        // TODO should this return a Result, so we can handle the situation the importer / imported
        // having shared descendants when as_packages=true?
        let mut temp_graph;
        let graph = match as_packages {
            true => {
                temp_graph = self.clone();
                temp_graph.squash_module(importer);
                temp_graph.squash_module(imported);
                &temp_graph
            }
            false => self,
        };
        graph.find_shortest_chain(importer, imported).is_some()
    }

    #[allow(unused_variables)]
    pub fn find_illegal_dependencies_for_layers(
        &self,
        levels: Vec<Level>,
        containers: HashSet<String>,
    ) -> Result<Vec<PackageDependency>, NoSuchContainer> {
        // Check that containers exist.
        let modules = self.get_modules();
        for container in containers.iter() {
            let container_module = Module::new(container.clone());
            if !modules.contains(&container_module) {
                return Err(NoSuchContainer {
                    container: container.clone(),
                });
            }
        }

        let all_layers: Vec<Module> = levels
            .iter()
            .flat_map(|level| level.layers.iter())
            .map(|module_name| Module::new(module_name.to_string()))
            .collect();
        
        let dependencies = self
            ._generate_module_permutations(&levels)
            .into_iter()
            //.into_par_iter()
            .filter_map(|(higher_layer_package, lower_layer_package)| {
                // TODO: it's inefficient to do this for sibling layers, as we don't need
                // to clone and trim the graph for identical pairs.
                info!(
                    "Searching for import chains from {} to {}...",
                    lower_layer_package, higher_layer_package
                );
                // let now = Instant::now();
                let dependency_or_none = self._search_for_package_dependency(
                    &higher_layer_package,
                    &lower_layer_package,
                    &all_layers,
                );
                // self._log_illegal_route_count(&dependency_or_none, now.elapsed().as_secs());
                dependency_or_none
            })
            .collect();

        Ok(dependencies)
    }

    // Return every permutation of modules that exist in the graph
    /// in which the second should not import the first.
    fn _generate_module_permutations(&self, levels: &Vec<Level>) -> Vec<(Module, Module)> {
        let mut permutations: Vec<(Module, Module)> = vec![];

        for (index, higher_level) in levels.iter().enumerate() {
            for higher_layer in &higher_level.layers {
                let higher_layer_module = Module {
                    name: higher_layer.clone(),
                };
                let mut layers_forbidden_to_import_higher_layer: Vec<Module> = vec![];
                for lower_level in &levels[index + 1..] {
                    for lower_layer in &lower_level.layers {
                        layers_forbidden_to_import_higher_layer
                            .push(Module::new(lower_layer.clone()));
                    }
                }

                // Add to permutations.
                for forbidden in layers_forbidden_to_import_higher_layer {
                    permutations.push((higher_layer_module.clone(), forbidden.clone()));
                }
            }
        }

        permutations
    }

    fn _search_for_package_dependency(
        &self,
        higher_layer_package: &Module,
        lower_layer_package: &Module,
        layers: &Vec<Module>,
    ) -> Option<PackageDependency> {
        let mut temp_graph = self.clone();

        // Remove other layers.
        let mut modules_to_remove: Vec<Module> = vec![];
        for layer in layers {
            if layer != higher_layer_package && layer != lower_layer_package {
                // Remove this subpackage.
                for descendant in temp_graph.find_descendants(&layer).unwrap() {
                    modules_to_remove.push(descendant.clone());
                }
                modules_to_remove.push(layer.clone());
            }
        }
        for module_to_remove in modules_to_remove.clone() {
            temp_graph.remove_module(&module_to_remove);
        }

        let mut routes: Vec<Route> = vec![];

        let direct_links= temp_graph._pop_direct_imports(
            lower_layer_package, higher_layer_package
        );
        for (importer, imported) in direct_links {
            routes.push(Route{
                heads: vec![importer],
                middle: vec![],
                tails: vec![imported],
            });
        }

        if routes.is_empty() {
            None
        } else {
            Some(PackageDependency{
                importer: lower_layer_package.clone(),
                imported: higher_layer_package.clone(),
                routes
            })
        }


    }

    /// Remove the direct imports, returning them as (importer, imported) tuples.
    fn _pop_direct_imports(
        &mut self,
        lower_layer_module: &Module,
        higher_layer_module: &Module,
    ) -> HashSet<(Module, Module)> {
        let mut imports = HashSet::new();

        let mut lower_layer_modules = HashSet::from([lower_layer_module.clone()]);
        for descendant in self.find_descendants(lower_layer_module).unwrap().iter().cloned() {
            lower_layer_modules.insert(descendant.clone());
        }

        let mut higher_layer_modules = HashSet::from([higher_layer_module.clone()]);
        for descendant in self.find_descendants(higher_layer_module).unwrap().iter().cloned() {
            higher_layer_modules.insert(descendant.clone());
        }

        for lower_layer_module in lower_layer_modules {
            for imported_module in self.find_modules_directly_imported_by(&lower_layer_module) {
                if higher_layer_modules.contains(imported_module) {
                    imports.insert((lower_layer_module.clone(), imported_module.clone()));
                }
            }

        }

        // Remove imports.
        for (importer, imported) in &imports {
            self.remove_import(&importer, &imported)
        }

        imports

    }

    #[allow(unused_variables)]
    pub fn squash_module(&mut self, module: &Module) {
        // Get descendants and their imports.
        let descendants: Vec<Module> = self
            .find_descendants(module)
            .unwrap()
            .into_iter()
            .cloned()
            .collect();
        let modules_imported_by_descendants: Vec<Module> = descendants
            .iter()
            .flat_map(|descendant| {
                self.find_modules_directly_imported_by(descendant)
                    .into_iter()
                    .cloned()
            })
            .collect();
        let modules_that_import_descendants: Vec<Module> = descendants
            .iter()
            .flat_map(|descendant| {
                self.find_modules_that_directly_import(descendant)
                    .into_iter()
                    .cloned()
            })
            .collect();

        // Remove any descendants.
        for descendant in descendants {
            self.remove_module(&descendant);
        }

        // Add descendants and imports to parent module.
        for imported in modules_imported_by_descendants {
            self.add_import(module, &imported);
        }

        for importer in modules_that_import_descendants {
            self.add_import(&importer, module);
        }

        self.squashed_modules.insert(module.clone());
    }

    pub fn is_module_squashed(&self, module: &Module) -> bool {
        self.squashed_modules.contains(module)
    }

    fn add_module_if_not_in_hierarchy(&mut self, module: &Module) {
        if self.hierarchy_module_indices.get_by_left(module).is_none() {
            self.add_module(module.clone());
        };
        if self.invisible_modules.contains(&module) {
            self.invisible_modules.remove(&module);
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::Level;

    #[test]
    fn modules_when_empty() {
        let graph = Graph::default();

        assert_eq!(graph.get_modules(), HashSet::new());
    }

    #[test]
    fn module_is_value_object() {
        assert_eq!(
            Module::new("mypackage".to_string()),
            Module::new("mypackage".to_string())
        );
    }

    #[test]
    fn add_module() {
        let mypackage = Module::new("mypackage".to_string());
        let mut graph = Graph::default();
        graph.add_module(mypackage.clone());

        let result = graph.get_modules();

        assert_eq!(result, HashSet::from([&mypackage]));
    }

    #[test]
    fn add_module_doesnt_add_parent() {
        let mypackage = Module::new("mypackage.foo".to_string());
        let mut graph = Graph::default();
        graph.add_module(mypackage.clone());

        let result = graph.get_modules();

        assert_eq!(result, HashSet::from([&mypackage]));
    }

    #[test]
    fn add_modules() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());

        let result = graph.get_modules();

        assert_eq!(result, HashSet::from([&mypackage, &mypackage_foo]));
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.foo
imports:

"
            .trim_start()
        );
    }

    #[test]
    fn remove_nonexistent_module() {
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mut graph = Graph::default();
        // Add mypackage but not mypackage.foo.
        graph.add_module(mypackage.clone());

        graph.remove_module(&mypackage_foo);

        let result = graph.get_modules();
        assert_eq!(result, HashSet::from([&mypackage]));
    }

    #[test]
    fn remove_existing_module_without_imports() {
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());

        let mut graph = Graph::default();
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_foo_alpha.clone());

        graph.remove_module(&mypackage_foo);

        let result = graph.get_modules();
        assert_eq!(
            result,
            HashSet::from([
                &mypackage,
                &mypackage_foo_alpha, // To be consistent with previous versions of Grimp.
            ])
        );
    }

    #[test]
    fn remove_existing_module_with_imports() {
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_import(&importer, &mypackage_foo);
        graph.add_import(&mypackage_foo, &imported);

        graph.remove_module(&mypackage_foo);

        let result = graph.get_modules();
        assert_eq!(
            result,
            HashSet::from([&mypackage, &mypackage_foo_alpha, &importer, &imported])
        );
        assert_eq!(
            graph.direct_import_exists(&importer, &mypackage_foo, false),
            false
        );
        assert_eq!(
            graph.direct_import_exists(&mypackage_foo, &imported, false),
            false
        );
    }

    #[test]
    fn remove_importer_module_removes_import_details() {
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();
        graph.add_detailed_import(&DetailedImport {
            importer: importer.clone(),
            imported: imported.clone(),
            line_number: 99,
            line_contents: "-".to_string(),
        });

        graph.remove_module(&importer);

        assert_eq!(
            graph.get_import_details(&importer, &imported),
            HashSet::new()
        );
    }

    #[test]
    fn remove_imported_module_removes_import_details() {
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();
        graph.add_detailed_import(&DetailedImport {
            importer: importer.clone(),
            imported: imported.clone(),
            line_number: 99,
            line_contents: "-".to_string(),
        });

        graph.remove_module(&imported);

        assert_eq!(
            graph.get_import_details(&importer, &imported),
            HashSet::new()
        );
    }

    #[test]
    fn remove_import_that_exists() {
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();
        graph.add_import(&importer, &imported);

        graph.remove_import(&importer, &imported);

        // The import has gone...
        assert_eq!(
            graph.direct_import_exists(&importer, &imported, false),
            false
        );
        // ...but the modules are still there.
        assert_eq!(graph.get_modules(), HashSet::from([&importer, &imported]));
    }

    #[test]
    fn remove_import_does_nothing_if_import_doesnt_exist() {
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();
        graph.add_module(importer.clone());
        graph.add_module(imported.clone());

        graph.remove_import(&importer, &imported);

        // The modules are still there.
        assert_eq!(graph.get_modules(), HashSet::from([&importer, &imported]));
    }

    #[test]
    fn remove_import_does_nothing_if_modules_dont_exist() {
        let importer = Module::new("importer".to_string());
        let imported = Module::new("importer".to_string());
        let mut graph = Graph::default();

        graph.remove_import(&importer, &imported);
    }

    #[test]
    fn remove_import_doesnt_affect_other_imports_from_same_modules() {
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        let yellow = Module::new("yellow".to_string());
        let red = Module::new("red".to_string());
        let mut graph = Graph::default();
        graph.add_import(&blue, &green);
        graph.add_import(&blue, &yellow);
        graph.add_import(&red, &blue);

        graph.remove_import(&blue, &green);

        // The other imports are still there.
        assert_eq!(graph.direct_import_exists(&blue, &yellow, false), true);
        assert_eq!(graph.direct_import_exists(&red, &blue, false), true);
    }

    #[test]
    #[should_panic(expected = "rootpackage is a root level package")]
    fn new_parent_root_module() {
        let root = Module::new("rootpackage".to_string());

        Module::new_parent(&root);
    }

    #[test]
    fn is_root_true() {
        let root = Module::new("rootpackage".to_string());

        assert!(root.is_root());
    }

    #[test]
    fn is_descendant_of_true_for_child() {
        let foo = Module::new("mypackage.foo".to_string());
        let foo_bar = Module::new("mypackage.foo.bar".to_string());

        assert!(foo_bar.is_descendant_of(&foo));
    }

    #[test]
    fn is_descendant_of_false_for_parent() {
        let foo = Module::new("mypackage.foo".to_string());
        let foo_bar = Module::new("mypackage.foo.bar".to_string());

        assert_eq!(foo.is_descendant_of(&foo_bar), false);
    }

    #[test]
    fn is_descendant_of_true_for_grandchild() {
        let foo = Module::new("mypackage.foo".to_string());
        let foo_bar_baz = Module::new("mypackage.foo.bar.baz".to_string());

        assert!(foo_bar_baz.is_descendant_of(&foo));
    }

    #[test]
    fn is_descendant_of_false_for_grandparent() {
        let foo = Module::new("mypackage.foo".to_string());
        let foo_bar_baz = Module::new("mypackage.foo.bar.baz".to_string());

        assert_eq!(foo.is_descendant_of(&foo_bar_baz), false);
    }

    #[test]
    fn is_root_false() {
        let non_root = Module::new("rootpackage.blue".to_string());

        assert_eq!(non_root.is_root(), false);
    }

    #[test]
    fn find_children_no_results() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());

        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());

        assert_eq!(graph.find_children(&mypackage_foo), HashSet::new());
    }

    #[test]
    fn find_children_one_result() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());

        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());

        assert_eq!(
            graph.find_children(&mypackage),
            HashSet::from([&mypackage_foo, &mypackage_bar])
        );
    }

    #[test]
    fn find_children_multiple_results() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());

        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());

        assert_eq!(
            graph.find_children(&mypackage),
            HashSet::from([&mypackage_foo, &mypackage_bar])
        );
    }

    #[test]
    fn find_children_returns_empty_set_with_nonexistent_module() {
        let mut graph = Graph::default();
        // Note: mypackage is not in the graph.
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());

        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());

        assert_eq!(
            graph.find_children(&Module::new("mypackage".to_string())),
            HashSet::new()
        );
    }

    #[test]
    fn find_descendants_no_results() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());

        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());

        assert_eq!(graph.find_descendants(&mypackage_bar), Ok(HashSet::new()));
    }

    #[test]
    fn find_descendants_module_not_in_graph() {
        let mut graph = Graph::default();
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        graph.add_module(blue.clone());

        assert_eq!(
            graph.find_descendants(&green),
            Err(ModuleNotPresent {
                module: green.clone()
            })
        );
    }

    #[test]
    fn find_descendants_multiple_results() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());

        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());

        assert_eq!(
            graph.find_descendants(&mypackage_foo),
            Ok(HashSet::from([
                &mypackage_foo_alpha,
                &mypackage_foo_alpha_blue,
                &mypackage_foo_alpha_green,
                &mypackage_foo_beta
            ]))
        );
    }

    #[test]
    fn find_descendants_with_gap() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        // mypackage.foo.blue is not added.
        let mypackage_foo_blue_alpha = Module::new("mypackage.foo.blue.alpha".to_string());
        let mypackage_foo_blue_alpha_one = Module::new("mypackage.foo.blue.alpha.one".to_string());
        let mypackage_foo_blue_alpha_two = Module::new("mypackage.foo.blue.alpha.two".to_string());
        let mypackage_foo_blue_beta_three =
            Module::new("mypackage.foo.blue.beta.three".to_string());
        let mypackage_bar_green_alpha = Module::new("mypackage.bar.green.alpha".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_foo_blue_alpha.clone());
        graph.add_module(mypackage_foo_blue_alpha_one.clone());
        graph.add_module(mypackage_foo_blue_alpha_two.clone());
        graph.add_module(mypackage_foo_blue_beta_three.clone());
        graph.add_module(mypackage_bar_green_alpha.clone());

        assert_eq!(
            graph.find_descendants(&mypackage_foo),
            // mypackage.foo.blue is not included.
            Ok(HashSet::from([
                &mypackage_foo_blue_alpha,
                &mypackage_foo_blue_alpha_one,
                &mypackage_foo_blue_alpha_two,
                &mypackage_foo_blue_beta_three,
            ]))
        );
    }

    #[test]
    fn find_descendants_added_in_different_order() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_foo_blue_alpha = Module::new("mypackage.foo.blue.alpha".to_string());
        let mypackage_foo_blue_alpha_one = Module::new("mypackage.foo.blue.alpha.one".to_string());
        let mypackage_foo_blue_alpha_two = Module::new("mypackage.foo.blue.alpha.two".to_string());
        let mypackage_foo_blue_beta_three =
            Module::new("mypackage.foo.blue.beta.three".to_string());
        let mypackage_bar_green_alpha = Module::new("mypackage.bar.green.alpha".to_string());
        let mypackage_foo_blue = Module::new("mypackage.foo.blue".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_foo_blue_alpha.clone());
        graph.add_module(mypackage_foo_blue_alpha_one.clone());
        graph.add_module(mypackage_foo_blue_alpha_two.clone());
        graph.add_module(mypackage_foo_blue_beta_three.clone());
        graph.add_module(mypackage_bar_green_alpha.clone());
        // Add the middle one at the end.
        graph.add_module(mypackage_foo_blue.clone());

        assert_eq!(
            graph.find_descendants(&mypackage_foo),
            Ok(HashSet::from([
                &mypackage_foo_blue, // Should be included.
                &mypackage_foo_blue_alpha,
                &mypackage_foo_blue_alpha_one,
                &mypackage_foo_blue_alpha_two,
                &mypackage_foo_blue_beta_three,
            ]))
        );
    }

    #[test]
    fn direct_import_exists_returns_true() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, false));
    }

    #[test]
    fn add_detailed_import_adds_import() {
        let mut graph = Graph::default();
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        let import = DetailedImport {
            importer: blue.clone(),
            imported: green.clone(),
            line_number: 11,
            line_contents: "-".to_string(),
        };

        graph.add_detailed_import(&import);

        assert_eq!(graph.direct_import_exists(&blue, &green, false), true);
    }

    #[test]
    fn direct_import_exists_returns_false() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert!(!graph.direct_import_exists(&mypackage_bar, &mypackage_foo, false));
    }

    #[test]
    fn direct_import_exists_returns_false_root_to_child() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_import(&mypackage_bar, &mypackage_foo_alpha);

        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.bar
    mypackage -> mypackage.foo
    mypackage.foo -> mypackage.foo.alpha
imports:
    mypackage.bar -> mypackage.foo.alpha
"
            .trim_start()
        );
        assert!(!graph.direct_import_exists(&mypackage_bar, &mypackage_foo, false));
    }

    #[test]
    fn add_import_with_non_existent_importer_adds_that_module() {
        let mut graph = Graph::default();
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage_bar.clone());

        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert_eq!(
            graph.get_modules(),
            HashSet::from([&mypackage_bar, &mypackage_foo])
        );
        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, false));
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    (mypackage) -> mypackage.bar
    (mypackage) -> mypackage.foo
imports:
    mypackage.foo -> mypackage.bar
"
            .trim_start()
        );
    }

    #[test]
    fn add_import_with_non_existent_imported_adds_that_module() {
        let mut graph = Graph::default();
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage_foo.clone());

        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert_eq!(
            graph.get_modules(),
            HashSet::from([&mypackage_bar, &mypackage_foo])
        );
        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, false));
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    (mypackage) -> mypackage.bar
    (mypackage) -> mypackage.foo
imports:
    mypackage.foo -> mypackage.bar
"
            .trim_start()
        );
    }

    #[test]
    fn direct_import_exists_with_as_packages_returns_false() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        // Add an import in the other direction.
        graph.add_import(&mypackage_bar, &mypackage_foo);

        assert!(!graph.direct_import_exists(&mypackage_foo, &mypackage_bar, true));
    }

    #[test]
    fn direct_import_exists_with_as_packages_returns_true_between_roots() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, true));
    }

    #[test]
    fn direct_import_exists_with_as_packages_returns_true_root_to_child() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        graph.add_import(&mypackage_bar, &mypackage_foo_alpha);

        assert!(graph.direct_import_exists(&mypackage_bar, &mypackage_foo, true));
    }

    #[test]
    fn direct_import_exists_with_as_packages_returns_true_child_to_root() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        graph.add_import(&mypackage_foo_alpha, &mypackage_bar);

        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, true));
    }

    #[test]
    #[should_panic]
    fn direct_import_exists_within_package_panics() {
        let mut graph = Graph::default();
        let ancestor = Module::new("mypackage.foo".to_string());
        let descendant = Module::new("mypackage.foo.blue.alpha".to_string());
        graph.add_import(&ancestor, &descendant);

        graph.direct_import_exists(&ancestor, &descendant, true);
    }

    #[test]
    fn find_modules_that_directly_import() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        let anotherpackage = Module::new("anotherpackage".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        graph.add_import(&mypackage_foo_alpha, &mypackage_bar);
        graph.add_import(&anotherpackage, &mypackage_bar);
        graph.add_import(&mypackage_bar, &mypackage_foo_alpha_green);

        let result = graph.find_modules_that_directly_import(&mypackage_bar);

        assert_eq!(
            result,
            HashSet::from([&mypackage_foo_alpha, &anotherpackage])
        )
    }

    #[test]
    fn find_modules_that_directly_import_after_removal() {
        let mut graph = Graph::default();
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        let yellow = Module::new("yellow".to_string());
        graph.add_import(&green, &blue);
        graph.add_import(&yellow, &blue);

        graph.remove_import(&green, &blue);
        let result = graph.find_modules_that_directly_import(&blue);

        assert_eq!(result, HashSet::from([&yellow]))
    }

    #[test]
    fn find_modules_directly_imported_by() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_foo_alpha = Module::new("mypackage.foo.alpha".to_string());
        let mypackage_foo_alpha_blue = Module::new("mypackage.foo.alpha.blue".to_string());
        let mypackage_foo_alpha_green = Module::new("mypackage.foo.alpha.green".to_string());
        let mypackage_foo_beta = Module::new("mypackage.foo.beta".to_string());
        let anotherpackage = Module::new("anotherpackage".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_module(mypackage_foo_alpha.clone());
        graph.add_module(mypackage_foo_alpha_blue.clone());
        graph.add_module(mypackage_foo_alpha_green.clone());
        graph.add_module(mypackage_foo_beta.clone());
        graph.add_import(&mypackage_bar, &mypackage_foo_alpha);
        graph.add_import(&mypackage_bar, &anotherpackage);
        graph.add_import(&mypackage_foo_alpha_green, &mypackage_bar);

        let result = graph.find_modules_directly_imported_by(&mypackage_bar);

        assert_eq!(
            result,
            HashSet::from([&mypackage_foo_alpha, &anotherpackage])
        )
    }

    #[test]
    fn find_modules_directly_imported_by_after_removal() {
        let mut graph = Graph::default();
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        let yellow = Module::new("yellow".to_string());
        graph.add_import(&blue, &green);
        graph.add_import(&blue, &yellow);

        graph.remove_import(&blue, &green);
        let result = graph.find_modules_directly_imported_by(&blue);

        assert_eq!(result, HashSet::from([&yellow]))
    }

    #[test]
    fn squash_module_descendants() {
        let mut graph = Graph::default();
        // Module we're going to squash.
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_blue = Module::new("mypackage.blue".to_string());
        let mypackage_blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let mypackage_blue_alpha_foo = Module::new("mypackage.blue.alpha.foo".to_string());
        let mypackage_blue_beta = Module::new("mypackage.blue.beta".to_string());
        // Other modules.
        let mypackage_green = Module::new("mypackage.green".to_string());
        let mypackage_red = Module::new("mypackage.red".to_string());
        let mypackage_orange = Module::new("mypackage.orange".to_string());
        let mypackage_yellow = Module::new("mypackage.yellow".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_blue.clone());
        // Module's descendants importing other modules.
        graph.add_import(&mypackage_blue_alpha, &mypackage_green);
        graph.add_import(&mypackage_blue_alpha, &mypackage_red);
        graph.add_import(&mypackage_blue_alpha_foo, &mypackage_yellow);
        graph.add_import(&mypackage_blue_beta, &mypackage_orange);
        // Other modules importing squashed module's descendants.
        graph.add_import(&mypackage_red, &mypackage_blue_alpha);
        graph.add_import(&mypackage_yellow, &mypackage_blue_alpha);
        graph.add_import(&mypackage_orange, &mypackage_blue_alpha_foo);
        graph.add_import(&mypackage_green, &mypackage_blue_beta);
        // Unrelated imports.
        graph.add_import(&mypackage_green, &mypackage_orange);
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.blue
    mypackage -> mypackage.green
    mypackage -> mypackage.orange
    mypackage -> mypackage.red
    mypackage -> mypackage.yellow
    mypackage.blue -> mypackage.blue.alpha
    mypackage.blue -> mypackage.blue.beta
    mypackage.blue.alpha -> mypackage.blue.alpha.foo
imports:
    mypackage.blue.alpha -> mypackage.green
    mypackage.blue.alpha -> mypackage.red
    mypackage.blue.alpha.foo -> mypackage.yellow
    mypackage.blue.beta -> mypackage.orange
    mypackage.green -> mypackage.blue.beta
    mypackage.green -> mypackage.orange
    mypackage.orange -> mypackage.blue.alpha.foo
    mypackage.red -> mypackage.blue.alpha
    mypackage.yellow -> mypackage.blue.alpha
"
            .trim_start()
        );

        graph.squash_module(&mypackage_blue);

        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.blue
    mypackage -> mypackage.green
    mypackage -> mypackage.orange
    mypackage -> mypackage.red
    mypackage -> mypackage.yellow
imports:
    mypackage.blue -> mypackage.green
    mypackage.blue -> mypackage.orange
    mypackage.blue -> mypackage.red
    mypackage.blue -> mypackage.yellow
    mypackage.green -> mypackage.blue
    mypackage.green -> mypackage.orange
    mypackage.orange -> mypackage.blue
    mypackage.red -> mypackage.blue
    mypackage.yellow -> mypackage.blue
"
            .trim_start()
        );
    }

    #[test]
    fn squash_module_no_descendants() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_blue = Module::new("mypackage.blue".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(mypackage_blue.clone());

        graph.squash_module(&mypackage_blue);

        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.blue
imports:

"
            .trim_start()
        );
    }

    #[test]
    fn find_count_imports_empty_graph() {
        let graph = Graph::default();

        let result = graph.count_imports();

        assert_eq!(result, 0);
    }

    #[test]
    fn find_count_imports_modules_but_no_imports() {
        let mut graph = Graph::default();
        graph.add_module(Module::new("mypackage.foo".to_string()));
        graph.add_module(Module::new("mypackage.bar".to_string()));

        let result = graph.count_imports();

        assert_eq!(result, 0);
    }

    #[test]
    fn find_count_imports_some_imports() {
        let mut graph = Graph::default();
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        let mypackage_baz = Module::new("mypackage.baz".to_string());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_import(&mypackage_foo, &mypackage_bar);
        graph.add_import(&mypackage_foo, &mypackage_baz);

        let result = graph.count_imports();

        assert_eq!(result, 2);
    }

    #[test]
    fn find_count_imports_treats_two_imports_between_same_modules_as_one() {
        let mut graph = Graph::default();
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());
        graph.add_import(&mypackage_foo, &mypackage_bar);
        graph.add_import(&mypackage_foo, &mypackage_bar);

        let result = graph.count_imports();

        assert_eq!(result, 1);
    }

    #[test]
    fn is_module_squashed_when_not_squashed() {
        let mut graph = Graph::default();
        // Module we're going to squash.
        let mypackage_blue = Module::new("mypackage.blue".to_string());
        let mypackage_blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        // Other module.
        let mypackage_green = Module::new("mypackage.green".to_string());
        graph.add_module(mypackage_blue.clone());
        graph.add_module(mypackage_blue_alpha.clone());
        graph.add_module(mypackage_green.clone());
        graph.squash_module(&mypackage_blue);

        let result = graph.is_module_squashed(&mypackage_green);

        assert!(!result);
    }

    #[test]
    fn is_module_squashed_when_squashed() {
        let mut graph = Graph::default();
        // Module we're going to squash.
        let mypackage_blue = Module::new("mypackage.blue".to_string());
        let mypackage_blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        // Other module.
        let mypackage_green = Module::new("mypackage.green".to_string());
        graph.add_module(mypackage_blue.clone());
        graph.add_module(mypackage_blue_alpha.clone());
        graph.add_module(mypackage_green.clone());
        graph.squash_module(&mypackage_blue);

        let result = graph.is_module_squashed(&mypackage_blue);

        assert!(result);
    }

    #[test]
    fn find_upstream_modules_when_there_are_some() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let yellow = Module::new("mypackage.yellow".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let brown = Module::new("mypackage.brown".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(yellow.clone());
        graph.add_module(purple.clone());
        graph.add_module(orange.clone());
        graph.add_module(brown.clone());
        // Add the import chain we care about.
        graph.add_import(&blue, &green);
        graph.add_import(&blue, &red);
        graph.add_import(&green, &yellow);
        graph.add_import(&yellow, &purple);
        // Add an import to blue.
        graph.add_import(&brown, &blue);

        let result = graph.find_upstream_modules(&blue, false);

        assert_eq!(result, HashSet::from([&green, &red, &yellow, &purple]))
    }

    #[test]
    fn find_upstream_modules_when_module_doesnt_exist() {
        let graph = Graph::default();
        let blue = Module::new("mypackage.blue".to_string());

        let result = graph.find_upstream_modules(&blue, false);

        assert_eq!(result, HashSet::new())
    }

    #[test]
    fn find_upstream_modules_as_packages() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let alpha = Module::new("mypackage.blue.alpha".to_string());
        let beta = Module::new("mypackage.blue.beta".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let yellow = Module::new("mypackage.yellow".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let brown = Module::new("mypackage.brown".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(alpha.clone());
        graph.add_module(beta.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(yellow.clone());
        graph.add_module(purple.clone());
        graph.add_module(orange.clone());
        graph.add_module(brown.clone());
        // Add the import chains we care about.
        graph.add_import(&blue, &green);
        graph.add_import(&green, &yellow);
        graph.add_import(&alpha, &purple);
        graph.add_import(&purple, &brown);
        // Despite being technically upstream, beta doesn't appear because it's
        // in the same package.
        graph.add_import(&purple, &beta);

        let result = graph.find_upstream_modules(&blue, true);

        assert_eq!(result, HashSet::from([&green, &yellow, &purple, &brown]))
    }

    #[test]
    fn find_downstream_modules_when_there_are_some() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let yellow = Module::new("mypackage.yellow".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let brown = Module::new("mypackage.brown".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(yellow.clone());
        graph.add_module(purple.clone());
        graph.add_module(orange.clone());
        graph.add_module(brown.clone());
        // Add the import chain we care about.
        graph.add_import(&blue, &green);
        graph.add_import(&blue, &red);
        graph.add_import(&green, &yellow);
        graph.add_import(&yellow, &purple);
        // Add an import from purple.
        graph.add_import(&purple, &brown);

        let result = graph.find_downstream_modules(&purple, false);

        assert_eq!(result, HashSet::from([&yellow, &green, &blue]))
    }

    #[test]
    fn find_downstream_modules_when_module_doesnt_exist() {
        let graph = Graph::default();
        let blue = Module::new("mypackage.blue".to_string());

        let result = graph.find_downstream_modules(&blue, false);

        assert_eq!(result, HashSet::new())
    }

    #[test]
    fn find_downstream_modules_as_packages() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let alpha = Module::new("mypackage.blue.alpha".to_string());
        let beta = Module::new("mypackage.blue.beta".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let yellow = Module::new("mypackage.yellow".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let brown = Module::new("mypackage.brown".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(alpha.clone());
        graph.add_module(beta.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(yellow.clone());
        graph.add_module(purple.clone());
        graph.add_module(orange.clone());
        graph.add_module(brown.clone());
        // Add the import chains we care about.
        graph.add_import(&yellow, &green);
        graph.add_import(&green, &blue);
        graph.add_import(&brown, &purple);
        graph.add_import(&purple, &alpha);
        // Despite being technically downstream, beta doesn't appear because it's
        // in the same package.
        graph.add_import(&beta, &yellow);

        let result = graph.find_downstream_modules(&blue, true);

        assert_eq!(result, HashSet::from([&green, &yellow, &purple, &brown]))
    }

    // find_shortest_chain
    #[test]
    fn find_shortest_chain_none() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(purple.clone());
        // Add imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chain(&blue, &green);

        assert!(result.is_none())
    }

    #[test]
    fn find_shortest_chain_one_step() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add the one-step chain.
        graph.add_import(&blue, &green);
        // Add a longer chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chain(&blue, &green).unwrap();

        assert_eq!(result, vec![&blue, &green])
    }

    #[test]
    fn find_shortest_chain_one_step_reverse() {
        let mut graph = Graph::default();
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        // Add the one-step chain.
        graph.add_import(&blue, &green);

        let result = graph.find_shortest_chain(&green, &blue);

        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn find_shortest_chain_two_steps() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(orange.clone());
        graph.add_module(purple.clone());
        // Add the two-step chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &green);
        // Add a longer chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &orange);
        graph.add_import(&orange, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chain(&blue, &green).unwrap();

        assert_eq!(result, vec![&blue, &red, &green])
    }

    #[test]
    fn find_shortest_chain_three_steps() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let orange = Module::new("mypackage.orange".to_string());
        let yellow = Module::new("mypackage.yellow".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(orange.clone());
        graph.add_module(yellow.clone());
        graph.add_module(purple.clone());
        // Add the three-step chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &orange);
        graph.add_import(&orange, &green);
        // Add a longer chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &orange);
        graph.add_import(&orange, &yellow);
        graph.add_import(&yellow, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chain(&blue, &green).unwrap();

        assert_eq!(result, vec![&blue, &red, &orange, &green])
    }

    // find_shortest_chains

    #[test]
    fn find_shortest_chains_none() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(purple.clone());
        // Add imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(result, HashSet::new())
    }

    #[test]
    fn find_shortest_chains_between_passed_modules() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(result, HashSet::from([vec![&blue, &red, &green],]))
    }

    #[test]
    fn find_shortest_chains_between_passed_module_and_child() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let green_alpha = Module::new("mypackage.green.alpha".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(green_alpha.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &green_alpha);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(result, HashSet::from([vec![&blue, &red, &green_alpha],]))
    }

    #[test]
    fn find_shortest_chains_between_passed_module_and_grandchild() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let green = Module::new("mypackage.green".to_string());
        let green_alpha = Module::new("mypackage.green.alpha".to_string());
        let green_alpha_one = Module::new("mypackage.green.alpha.one".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(green.clone());
        graph.add_module(green_alpha.clone());
        graph.add_module(green_alpha_one.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue, &red);
        graph.add_import(&red, &green_alpha_one);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(
            result,
            HashSet::from([vec![&blue, &red, &green_alpha_one],])
        )
    }

    #[test]
    fn find_shortest_chains_between_child_and_passed_module() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha, &red);
        graph.add_import(&red, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(result, HashSet::from([vec![&blue_alpha, &red, &green],]))
    }

    #[test]
    fn find_shortest_chains_between_grandchild_and_passed_module() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let blue_alpha_one = Module::new("mypackage.blue.alpha.one".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(blue_alpha_one.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha_one, &red);
        graph.add_import(&red, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.find_shortest_chains(&blue, &green);

        assert_eq!(
            result,
            HashSet::from([vec![&blue_alpha_one, &red, &green],])
        )
    }

    #[test]
    fn chain_exists_true_as_packages_false() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let blue_alpha_one = Module::new("mypackage.blue.alpha.one".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(blue_alpha_one.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha_one, &red);
        graph.add_import(&red, &green);
        // Add other imports that are irrelevant.
        graph.add_import(&purple, &blue);
        graph.add_import(&green, &purple);

        let result = graph.chain_exists(&blue_alpha_one, &green, false);

        assert!(result);
    }

    #[test]
    fn chain_exists_false_as_packages_false() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let blue_alpha_one = Module::new("mypackage.blue.alpha.one".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(blue_alpha_one.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha_one, &red);
        graph.add_import(&red, &green);

        let result = graph.chain_exists(&blue, &green, false);

        assert_eq!(result, false);
    }

    #[test]
    fn chain_exists_true_as_packages_true() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let blue_alpha_one = Module::new("mypackage.blue.alpha.one".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(blue_alpha_one.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha_one, &red);
        graph.add_import(&red, &green);

        let result = graph.chain_exists(&blue, &green, true);

        assert_eq!(result, true);
    }

    #[test]
    fn chain_exists_false_as_packages_true() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let blue = Module::new("mypackage.blue".to_string());
        let blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let blue_alpha_one = Module::new("mypackage.blue.alpha.one".to_string());
        let green = Module::new("mypackage.green".to_string());
        let red = Module::new("mypackage.red".to_string());
        let purple = Module::new("mypackage.purple".to_string());
        graph.add_module(mypackage.clone());
        graph.add_module(blue.clone());
        graph.add_module(blue_alpha.clone());
        graph.add_module(blue_alpha_one.clone());
        graph.add_module(green.clone());
        graph.add_module(red.clone());
        graph.add_module(purple.clone());
        // Add a chain.
        graph.add_import(&blue_alpha_one, &red);
        graph.add_import(&red, &green);

        let result = graph.chain_exists(&green, &blue, true);

        assert_eq!(result, false);
    }

    #[test]
    fn find_illegal_dependencies_for_layers_empty_everything() {
        let graph = Graph::default();

        let dependencies = graph.find_illegal_dependencies_for_layers(vec![], HashSet::new());

        assert_eq!(dependencies, Ok(vec![]));
    }

    #[test]
    fn find_illegal_dependencies_for_layers_no_such_container() {
        let graph = Graph::default();
        let container = "nonexistent_container".to_string();

        let dependencies =
            graph.find_illegal_dependencies_for_layers(vec![], HashSet::from([container.clone()]));

        assert_eq!(
            dependencies,
            Err(NoSuchContainer {
                container: container
            })
        );
    }

    #[test]
    fn find_illegal_dependencies_for_layers_nonexistent_layers_no_container() {
        let graph = Graph::default();
        let level = Level {
            layers: vec!["nonexistent".to_string()],
            independent: true,
        };

        let dependencies = graph.find_illegal_dependencies_for_layers(vec![level], HashSet::new());

        assert_eq!(dependencies, Ok(vec![]));
    }

    #[test]
    fn find_illegal_dependencies_for_layers_nonexistent_layers_with_container() {
        let mut graph = Graph::default();
        graph.add_module(Module::new("mypackage".to_string()));
        let level = Level {
            layers: vec!["nonexistent".to_string()],
            independent: true,
        };
        let container = "mypackage".to_string();

        let dependencies =
            graph.find_illegal_dependencies_for_layers(vec![level], HashSet::from([container]));

        assert_eq!(dependencies, Ok(vec![]));
    }

    #[test]
    fn find_illegal_dependencies_for_layers_no_container_one_illegal_dependency() {
        let mut graph = Graph::default();
        let high = Module::new("high".to_string());
        let low = Module::new("low".to_string());
        graph.add_import(&low, &high);
        let levels = vec![
            Level {
                layers: vec![high.name.clone()],
                independent: true,
            },
            Level {
                layers: vec![low.name.clone()],
                independent: true,
            },
        ];

        let dependencies = graph.find_illegal_dependencies_for_layers(levels, HashSet::new());

        assert_eq!(
            dependencies,
            Ok(vec![PackageDependency {
                importer: low.clone(),
                imported: high.clone(),
                routes: vec![Route {
                    heads: vec![low.clone()],
                    middle: vec![],
                    tails: vec![high.clone()],
                }]
            }])
        );
    }

    #[test]
    fn get_import_details_no_modules() {
        let graph = Graph::default();
        let importer = Module::new("foo".to_string());
        let imported = Module::new("bar".to_string());

        let result = graph.get_import_details(&importer, &imported);

        assert_eq!(result, HashSet::new());
    }

    #[test]
    fn get_import_details_module_without_metadata() {
        let mut graph = Graph::default();
        let importer = Module::new("foo".to_string());
        let imported = Module::new("bar".to_string());
        graph.add_import(&importer, &imported);

        let result = graph.get_import_details(&importer, &imported);

        assert_eq!(result, HashSet::new());
    }

    #[test]
    fn get_import_details_module_one_result() {
        let mut graph = Graph::default();
        let importer = Module::new("foo".to_string());
        let imported = Module::new("bar".to_string());
        let import = DetailedImport {
            importer: importer.clone(),
            imported: imported.clone(),
            line_number: 5,
            line_contents: "import bar".to_string(),
        };
        let unrelated_import = DetailedImport {
            importer: importer.clone(),
            imported: Module::new("baz".to_string()),
            line_number: 2,
            line_contents: "-".to_string(),
        };
        graph.add_detailed_import(&import);
        graph.add_detailed_import(&unrelated_import);

        let result = graph.get_import_details(&importer, &imported);

        assert_eq!(result, HashSet::from([import]));
    }

    #[test]
    fn get_import_details_module_two_results() {
        let mut graph = Graph::default();
        let blue = Module::new("blue".to_string());
        let green = Module::new("green".to_string());
        let blue_to_green_a = DetailedImport {
            importer: blue.clone(),
            imported: green.clone(),
            line_number: 5,
            line_contents: "import green".to_string(),
        };
        let blue_to_green_b = DetailedImport {
            importer: blue.clone(),
            imported: green.clone(),
            line_number: 15,
            line_contents: "import green".to_string(),
        };
        graph.add_detailed_import(&blue_to_green_a);
        graph.add_detailed_import(&blue_to_green_b);

        let result = graph.get_import_details(&blue, &green);

        assert_eq!(result, HashSet::from([blue_to_green_a, blue_to_green_b]));
    }

    #[test]
    fn get_import_details_after_removal() {
        let mut graph = Graph::default();
        let importer = Module::new("foo".to_string());
        let imported = Module::new("bar".to_string());
        let import = DetailedImport {
            importer: importer.clone(),
            imported: imported.clone(),
            line_number: 5,
            line_contents: "import bar".to_string(),
        };
        let unrelated_import = DetailedImport {
            importer: importer.clone(),
            imported: Module::new("baz".to_string()),
            line_number: 2,
            line_contents: "-".to_string(),
        };
        graph.add_detailed_import(&import);
        graph.add_detailed_import(&unrelated_import);
        graph.remove_import(&import.importer, &import.imported);

        let result = graph.get_import_details(&importer, &imported);

        assert_eq!(result, HashSet::new());
    }

    #[test]
    fn get_import_details_after_removal_of_unrelated_import() {
        let mut graph = Graph::default();
        let importer = Module::new("foo".to_string());
        let imported = Module::new("bar".to_string());
        let import = DetailedImport {
            importer: importer.clone(),
            imported: imported.clone(),
            line_number: 5,
            line_contents: "import bar".to_string(),
        };
        let unrelated_import = DetailedImport {
            importer: importer.clone(),
            imported: Module::new("baz".to_string()),
            line_number: 2,
            line_contents: "-".to_string(),
        };
        graph.add_detailed_import(&import);
        graph.add_detailed_import(&unrelated_import);
        graph.remove_import(&unrelated_import.importer, &unrelated_import.imported);

        let result = graph.get_import_details(&importer, &imported);

        assert_eq!(result, HashSet::from([import]));
    }
}
