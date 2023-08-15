use _rustgrimp::importgraph::ImportGraph;
use _rustgrimp::layers::{find_illegal_dependencies, Level};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;

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

    let levels = vec![
        Level {
            layers: vec!["plugins"],
        },
        Level {
            layers: vec!["interfaces"],
        },
        Level {
            layers: vec!["application"],
        },
        Level {
            layers: vec!["domain"],
        },
        Level {
            layers: vec!["data"],
        },
    ];
    let containers = HashSet::from(["mypackage"]);

    find_illegal_dependencies(&graph, &levels, &containers);
}
