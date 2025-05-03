use _rustgrimp::graph::Graph;
use _rustgrimp::graph::higher_order_queries::Level;
use rustc_hash::FxHashSet;
use serde_json::{Map, Value};
use std::fs;
use tap::Conv;

#[test]
fn test_large_graph_deep_layers() {
    let data = fs::read_to_string("tests/large_graph.json").expect("Unable to read file");
    let value: Value = serde_json::from_str(&data).unwrap();
    let items: &Map<String, Value> = value.as_object().unwrap();

    let mut graph = Graph::default();
    for (importer, importeds_value) in items.iter() {
        let importer = graph.get_or_add_module(importer).token();
        for imported in importeds_value.as_array().unwrap() {
            let imported = graph.get_or_add_module(imported.as_str().unwrap()).token();
            graph.add_import(importer, imported);
        }
    }

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
        .into_iter()
        .map(|layer| {
            Level::new(
                graph
                    .get_module_by_name(layer)
                    .unwrap()
                    .token()
                    .conv::<FxHashSet<_>>(),
                true,
            )
        })
        .collect();

    let deps = graph.find_illegal_dependencies_for_layers(&levels).unwrap();

    assert_eq!(deps.len(), 8);
}
