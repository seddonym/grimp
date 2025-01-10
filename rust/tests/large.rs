use _rustgrimp::importgraph::ImportGraph;
use _rustgrimp::layers::{find_illegal_dependencies, Level};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;

#[test]
fn test_large_graph_deep_layers() {
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

    let deep_layers = vec![
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1991886645",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.6397984863",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.9009030339",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.6666171185",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1693068682",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.1752284225",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.9089085203",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.5033127033",
        "mypackage.plugins.5634303718.1007553798.8198145119.application.3242334296.2454157946",
    ];
    let levels: Vec<Level> = deep_layers
        .iter()
        .map(|layer| Level {
            independent: true,
            layers: vec![layer.to_string()],
        })
        .collect();
    let containers = HashSet::new();

    let deps = find_illegal_dependencies(&graph, &levels, &containers);

    assert_eq!(deps.len(), 8);
}
