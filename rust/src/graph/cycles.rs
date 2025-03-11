/*
TODO K4liber: description

Finds all cycles using Johnson's algorithm.
*/
use std::collections::{HashSet, HashMap};

use crate::graph::Graph;

struct _DirectedGraph {
    adj_list: HashMap<String, Vec<String>>,
}

impl _DirectedGraph {

    fn new() -> Self {
        _DirectedGraph {
            adj_list: HashMap::new()
        }
    }

    fn add_edge(&mut self, u: &str, v: &str) {
        self.adj_list
        .entry(u.to_string())
        .or_insert_with(Vec::new)
        .push(v.to_string());
    }

    fn find_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut blocked = HashSet::new();
        let mut stack = Vec::new();
        let mut b_sets: HashMap<String, HashSet<String>> = HashMap::new();

        for start in self.adj_list.keys() {
            let mut subgraph = self.build_subgraph(start);
            self.find_cycles_from(
                start,
                start,
                &mut blocked,
                &mut stack,
                &mut b_sets,
                &mut cycles,
                &mut subgraph,
            );
        }

        cycles
    }

    fn build_subgraph(&self, start: &String) -> HashMap<String, Vec<String>> {
        self.adj_list
            .iter()
            .filter(|(node, _)| node >= &start)
            .map(|(node, neighbors)| {
                let filtered_neighbors: Vec<String> =
                    neighbors.iter().filter(|n| n >= &start).cloned().collect();
                (node.clone(), filtered_neighbors)
            })
            .collect()
    }

    fn find_cycles_from(
        &self,
        start: &String,
        v: &String,
        blocked: &mut HashSet<String>,
        stack: &mut Vec<String>,
        b_sets: &mut HashMap<String, HashSet<String>>,
        cycles: &mut Vec<Vec<String>>,
        subgraph: &mut HashMap<String, Vec<String>>,
    ) -> bool {
        let mut found_cycle = false;
        stack.push(v.clone());
        blocked.insert(v.clone());

        let neighbors = subgraph.get(v).cloned().unwrap_or_default();
        for w in neighbors {
            if &w == start {
                cycles.push(stack.clone());
                found_cycle = true;
            } else if !blocked.contains(&w) {
                if self.find_cycles_from(start, &w, blocked, stack, b_sets, cycles, subgraph) {
                    found_cycle = true;
                }
            }
        }

        if found_cycle {
            self.unblock(v, blocked, b_sets);
        } else {
            if let Some(neighbors) = subgraph.get(v) {
                for w in neighbors {
                    b_sets.entry(w.clone()).or_default().insert(v.clone());
                }
            }
        }

        stack.pop();
        found_cycle
    }

    fn unblock(&self, node: &String, blocked: &mut HashSet<String>, b_sets: &mut HashMap<String, HashSet<String>>) {
        blocked.remove(node);
        let mut stack = vec![node.clone()];

        while let Some(n) = stack.pop() {
            if let Some(dependent_nodes) = b_sets.remove(&n) {
                for w in dependent_nodes {
                    if blocked.remove(&w) {
                        stack.push(w);
                    }
                }
            }
        }
    }
}

impl Graph {
    pub fn find_cycles(
        &self
    ) -> Vec<Vec<String>> {
        let mut directed_graph = _DirectedGraph::new();
        
        for module in self.all_modules() {
            let module_token = module.token();
            let imports_modules_tokens = self.imports.get(module_token).unwrap().iter();

            for next_module in imports_modules_tokens {
                let imported_module = self.get_module(*next_module);
                directed_graph.add_edge(module.name().as_str(), imported_module.unwrap().name().as_str());
            }
        }

        let cycles = directed_graph.find_cycles();
        return cycles;
    }
}
