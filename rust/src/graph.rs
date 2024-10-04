/*
modules (get_modules) DONE
find_children - DONE
find_descendants - DONE
direct_import_exists - DONE
find_modules_directly_imported_by - DONE
find_modules_that_directly_import -  DONE
get_import_details - DONE
count_imports - DONE
    find_downstream_modules - partially done - need to add as_package
    find_upstream_modules - partially done - need to add as_package
find_shortest_chain - DONE
    find_shortest_chains - TODO
chain_exists - DONE
    find_illegal_dependencies_for_layers - TODO
    add_module - PARTIALLY DONE - need to add is_squashed
remove_module - DONE
add_import - DONE
remove_import - DONE
squash_module - DONE
is_module_squashed - DONE

Also, sensible behaviour when passing modules that don't exist in the graph.
*/
#![allow(dead_code)]

use bimap::BiMap;
use petgraph::algo::astar;
use petgraph::graph::EdgeIndex;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::{Bfs, Walker};
use petgraph::Direction;
use std::collections::{HashSet, HashMap};
use std::fmt;

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
    detailed_imports_map: HashMap<(Module, Module), HashSet<DetailedImport>>,
}

impl Graph {
    pub fn pretty_str(&self) -> String {
        let mut hierarchy: Vec<String> = vec![];
        let mut imports: Vec<String> = vec![];

        let hierarchy_module_indices: Vec<_> = self.hierarchy_module_indices.iter().collect();

        for (from_module, from_index) in hierarchy_module_indices {
            for to_index in self.hierarchy.neighbors(*from_index) {
                let to_module = self
                    .hierarchy_module_indices
                    .get_by_right(&to_index)
                    .unwrap();
                hierarchy.push(format!("    {} -> {}", from_module.name, to_module.name));
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
        let module_index = self.hierarchy.add_node(module.clone());
        self.hierarchy_module_indices
            .insert(module.clone(), module_index);

        // Add to the hierarchy from the module's parent, if it has one.
        if !module.is_root() {
            let parent = Module::new_parent(&module);

            // If the parent isn't already in the graph, add it.
            let parent_index = match self.hierarchy_module_indices.get_by_left(&parent) {
                Some(index) => index,
                None => {
                    self.add_module(parent.clone());
                    self.hierarchy_module_indices.get_by_left(&parent).unwrap()
                }
            };

            self.hierarchy.add_edge(*parent_index, module_index, ());
        }
    }

    pub fn remove_module(&mut self, module: &Module) {
        if let Some(hierarchy_index) = self.hierarchy_module_indices.get_by_left(module) {
            self.hierarchy.remove_node(*hierarchy_index);
            self.hierarchy_module_indices.remove_by_left(module);
        };

        if let Some(imports_index) = self.imports_module_indices.get_by_left(module) {
            self.imports.remove_node(*imports_index);
            self.imports_module_indices.remove_by_left(module);
        };
    }

    pub fn get_modules(&self) -> HashSet<&Module> {
        self.hierarchy_module_indices.left_values().collect()
    }

    pub fn count_imports(&self) -> usize {
        self.imports.edge_count()
    }

    pub fn get_import_details(&self, importer: &Module, imported: &Module) -> HashSet<DetailedImport> {
        let key = (importer.clone(), imported.clone());
        match self.detailed_imports_map.get(&key) {
            Some(import_details) => import_details.clone(),
            None => HashSet::new(),
        }
    }

    pub fn find_children(&self, module: &Module) -> HashSet<&Module> {
        let module_index = self.hierarchy_module_indices.get_by_left(module).unwrap();
        self.hierarchy
            .neighbors(*module_index)
            .map(|index| self.hierarchy_module_indices.get_by_right(&index).unwrap())
            .collect()
    }

    pub fn find_descendants(&self, module: &Module) -> HashSet<&Module> {
        let module_index = self.hierarchy_module_indices.get_by_left(module).unwrap();
        Bfs::new(&self.hierarchy, *module_index)
            .iter(&self.hierarchy)
            .filter(|index| index != module_index) // Don't include the supplied module.
            .map(|index| self.hierarchy_module_indices.get_by_right(&index).unwrap())
            .collect()
    }

    pub fn add_import(&mut self, importer: &Module, imported: &Module) {
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
        self.detailed_imports_map.entry(key)
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
        self.imports_module_indices.remove_by_left(importer);
        self.imports_module_indices.remove_by_left(importer);
        let key = (importer.clone(), imported.clone());

        self.detailed_imports_map.remove(&key);
        self.imports.remove_edge(edge_index);
    }

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

    pub fn find_downstream_modules(&self, module: &Module) -> HashSet<&Module> {
        let module_index = match self.imports_module_indices.get_by_left(module) {
            Some(index) => *index,
            None => return HashSet::new(),
        };
        Bfs::new(&self.imports, module_index)
            .iter(&self.imports)
            .filter(|index| *index != module_index) // Don't include the supplied module.
            .map(|index| self.imports_module_indices.get_by_right(&index).unwrap())
            .collect()
    }

    pub fn find_upstream_modules(&self, module: &Module) -> HashSet<&Module> {
        let module_index = match self.imports_module_indices.get_by_left(module) {
            Some(index) => *index,
            None => return HashSet::new(),
        };

        // Reverse all the edges in the graph and then do what we do in find_downstream_modules.
        // Is there a way of doing this without the clone?
        let mut reversed_graph = self.imports.clone();
        reversed_graph.reverse();

        Bfs::new(&reversed_graph, module_index)
            .iter(&reversed_graph)
            .filter(|index| *index != module_index) // Don't include the supplied module.
            .map(|index| self.imports_module_indices.get_by_right(&index).unwrap())
            .collect()
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
        for descendant in self.find_descendants(&importer) {
            importer_modules.insert(descendant);
        }

        let mut imported_modules: HashSet<&Module> = HashSet::from([imported]);
        // TODO don't do this if module is squashed?
        for descendant in self.find_descendants(&imported) {
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
    pub fn chain_exists(
        &self,
        importer: &Module,
        imported: &Module,
        as_packages: bool,
    ) -> bool {
        let mut temp_graph;
        let graph = match as_packages {
            true => {
                temp_graph = self.clone();
                temp_graph.squash_module(importer);
                temp_graph.squash_module(imported);
                &temp_graph
            },
            false => self,
        };
        graph.find_shortest_chain(importer, imported).is_some()
    }

    #[allow(unused_variables)]
    pub fn squash_module(&mut self, module: &Module) {
        // Get descendants and their imports.
        let descendants: Vec<Module> = self.find_descendants(module).into_iter().cloned().collect();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn find_children_works_when_adding_orphans() {
        let mut graph = Graph::default();
        // Note: mypackage is not in the graph.
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());

        graph.add_module(mypackage_foo.clone());
        graph.add_module(mypackage_bar.clone());

        assert_eq!(
            graph.find_children(&Module::new("mypackage".to_string())),
            HashSet::from([&mypackage_foo, &mypackage_bar])
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

        assert_eq!(graph.find_descendants(&mypackage_bar), HashSet::new());
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
            HashSet::from([
                &mypackage_foo_alpha,
                &mypackage_foo_alpha_blue,
                &mypackage_foo_alpha_green,
                &mypackage_foo_beta
            ])
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
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage_bar.clone());

        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert_eq!(
            graph.get_modules(),
            HashSet::from([&mypackage, &mypackage_bar, &mypackage_foo])
        );
        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, false));
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.bar
    mypackage -> mypackage.foo
imports:
    mypackage.foo -> mypackage.bar
"
                .trim_start()
        );
    }

    #[test]
    fn add_import_with_non_existent_imported_adds_that_module() {
        let mut graph = Graph::default();
        let mypackage = Module::new("mypackage".to_string());
        let mypackage_foo = Module::new("mypackage.foo".to_string());
        let mypackage_bar = Module::new("mypackage.bar".to_string());
        graph.add_module(mypackage_foo.clone());

        graph.add_import(&mypackage_foo, &mypackage_bar);

        assert_eq!(
            graph.get_modules(),
            HashSet::from([&mypackage, &mypackage_bar, &mypackage_foo])
        );
        assert!(graph.direct_import_exists(&mypackage_foo, &mypackage_bar, false));
        assert_eq!(
            graph.pretty_str(),
            "
hierarchy:
    mypackage -> mypackage.bar
    mypackage -> mypackage.foo
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
    fn squash_module_descendants() {
        let mut graph = Graph::default();
        // Module we're going to squash.
        //let mypackage = Module::new("mypackage".to_string());
        let mypackage_blue = Module::new("mypackage.blue".to_string());
        let mypackage_blue_alpha = Module::new("mypackage.blue.alpha".to_string());
        let mypackage_blue_alpha_foo = Module::new("mypackage.blue.alpha.foo".to_string());
        let mypackage_blue_beta = Module::new("mypackage.blue.beta".to_string());
        // Other modules.
        let mypackage_green = Module::new("mypackage.green".to_string());
        let mypackage_red = Module::new("mypackage.red".to_string());
        let mypackage_orange = Module::new("mypackage.orange".to_string());
        let mypackage_yellow = Module::new("mypackage.yellow".to_string());
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
        let mypackage_blue = Module::new("mypackage.blue".to_string());
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
        // Add an import to blue.
        graph.add_import(&brown, &blue);

        let result = graph.find_downstream_modules(&blue);

        assert_eq!(result, HashSet::from([&green, &red, &yellow, &purple]))
    }

    #[test]
    fn find_downstream_modules_when_module_doesnt_exist() {
        let graph = Graph::default();
        let blue = Module::new("mypackage.blue".to_string());

        let result = graph.find_downstream_modules(&blue);

        assert_eq!(result, HashSet::new())
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
        // Add an import from purple.
        graph.add_import(&purple, &brown);

        let result = graph.find_upstream_modules(&purple);

        assert_eq!(result, HashSet::from([&yellow, &green, &blue]))
    }

    #[test]
    fn find_upstream_modules_when_module_doesnt_exist() {
        let graph = Graph::default();
        let blue = Module::new("mypackage.blue".to_string());

        let result = graph.find_upstream_modules(&blue);

        assert_eq!(result, HashSet::new())
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
