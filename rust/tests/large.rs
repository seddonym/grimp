use _rustgrimp::graph::{Graph, Level, Module};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs;

#[test]
fn test_large_graph() {
    let data = fs::read_to_string("tests/large_graph.json").expect("Unable to read file");
    let value: Value = serde_json::from_str(&data).unwrap();
    let items: &Map<String, Value> = value.as_object().unwrap();
    let mut graph = Graph::default();
    for (importer, importeds_value) in items.iter() {
        for imported in importeds_value.as_array().unwrap() {
            graph.add_import(
                &Module {
                    name: importer.to_string(),
                },
                &Module {
                    name: imported.to_string(),
                },
            );
        }
    }
    // TODO: for some reason this isn't populated in large_graph.json. Possibly there are other
    // things missing too, which may be why find_illegal_dependencies_for_layers doesn't currently
    // return any illegal dependencies.
    graph.add_module(Module{name: "mypackage".to_string()});

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

    graph.find_illegal_dependencies_for_layers(levels, containers).unwrap();
}
