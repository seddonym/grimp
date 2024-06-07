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
            independent: true,
            layers: vec!["plugins".to_string()],
        },
        Level {
            independent: true,
            layers: vec!["interfaces".to_string()],
        },
        Level {
            independent: true,
            layers: vec!["application".to_string()],
        },
        Level {
            independent: true,
            layers: vec!["domain".to_string()],
        },
        Level {
            independent: true,
            layers: vec!["data".to_string()],
        },
    ];
    let containers = HashSet::from(["mypackage".to_string()]);

    find_illegal_dependencies(&graph, &levels, &containers);
}
