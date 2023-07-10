use _rustgrimp::layers::find_illegal_dependencies;
use std::fs;
use serde_json::{Value, Map};
use _rustgrimp::importgraph::ImportGraph;
use std::collections::{HashMap, HashSet};

#[test]
fn test_large_graph() {
    let data = fs::read_to_string("tests/large_graph.json").expect("Unable to read file");
    let value: Value = serde_json::from_str(&data).unwrap();
    let items: &Map<String, Value> = value.as_object().unwrap();
    let mut importeds_by_importer: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (importer, importeds_value) in items.iter() {
        let mut importeds = HashSet::new();
        for imported in importeds_value.as_array().unwrap() {
            importeds.insert(imported.as_str().unwrap());
        }
        importeds_by_importer.insert(importer, importeds);
    }
    let graph = ImportGraph::new(importeds_by_importer);

    let layers = vec!["plugins", "interfaces", "application", "domain", "data"];
    let containers = HashSet::from(["mypackage"]);

    find_illegal_dependencies(&graph, &layers, &containers);
}