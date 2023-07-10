use crate::importgraph::ImportGraph;
use std::collections::HashSet;


pub fn check_containers_exist<'a>(
    graph: &'a ImportGraph,
    containers: &'a HashSet<&'a str>,
) -> Result<(), String> {
    for container in containers {
        if !graph.contains_module(container) {
            return Err(format!("Container {} does not exist.", container));
        }
    }
    Ok(())
}
