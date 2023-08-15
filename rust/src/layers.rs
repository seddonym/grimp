use crate::dependencies::{PackageDependency, Route};
use crate::importgraph::ImportGraph;

use log::info;
use std::collections::HashSet;
use std::time::Instant;

/// A group of layers at the same level in the layering.
/// These layers should be independent.
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Level<'a> {
    pub layers: Vec<&'a str>,
}

pub fn find_illegal_dependencies<'a>(
    graph: &'a ImportGraph,
    levels: &'a Vec<Level>,
    containers: &'a HashSet<&'a str>,
) -> Vec<PackageDependency> {
    let mut dependencies: Vec<PackageDependency> = vec![];
    let layers = _layers_from_levels(levels);

    for (higher_layer_package, lower_layer_package, container) in
        _generate_module_permutations(graph, levels, containers)
    {
        // TODO: it's inefficient to do this for sibling layers, as we don't need
        // to clone and trim the graph for identical pairs.
        info!(
            "Searching for import chains from {} to {}...",
            lower_layer_package, higher_layer_package
        );
        let now = Instant::now();
        let dependency_or_none = _search_for_package_dependency(
            &higher_layer_package,
            &lower_layer_package,
            &layers,
            &container,
            graph,
        );
        _log_illegal_route_count(&dependency_or_none, now.elapsed().as_secs());
        if let Some(dependency) = dependency_or_none {
            dependencies.push(dependency);
        }
    }
    dependencies
}

fn _generate_module_permutations<'a>(
    graph: &'a ImportGraph,
    levels: &'a [Level],
    containers: &'a HashSet<&'a str>,
) -> Vec<(String, String, Option<String>)> {
    let mut permutations: Vec<(String, String, Option<String>)> = vec![];

    let quasi_containers: Vec<Option<String>> = if containers.is_empty() {
        vec![None]
    } else {
        containers.iter().map(|i| Some(i.to_string())).collect()
    };
    for container in quasi_containers {
        for (index, higher_level) in levels.iter().enumerate() {
            for higher_layer in &higher_level.layers {
                let higher_layer_module_name = _module_from_layer(higher_layer, &container);
                if graph
                    .ids_by_name
                    .get(&higher_layer_module_name as &str)
                    .is_none()
                {
                    continue;
                }

                // Build the layers that mustn't import this higher layer. That
                // includes lower layers and siblings.
                let mut layers_forbidden_to_import_higher_layer: Vec<&str> = vec![];
                for potential_sibling_layer in &higher_level.layers {
                    if potential_sibling_layer != higher_layer {
                        // It's a sibling layer.
                        layers_forbidden_to_import_higher_layer.push(potential_sibling_layer);
                    }
                }

                for lower_level in &levels[index + 1..] {
                    for lower_layer in &lower_level.layers {
                        layers_forbidden_to_import_higher_layer.push(lower_layer);
                    }
                }

                // Now turn the layers into modules, if they exist.
                for forbidden_layer in &layers_forbidden_to_import_higher_layer {
                    let forbidden_module_name = _module_from_layer(forbidden_layer, &container);
                    if let Some(_value) = graph.ids_by_name.get(&forbidden_module_name as &str) {
                        permutations.push((
                            higher_layer_module_name.clone(),
                            forbidden_module_name.clone(),
                            container.clone(),
                        ));
                    };
                }
            }
        }
    }

    permutations
}

fn _module_from_layer<'a>(module: &'a str, container: &'a Option<String>) -> String {
    match container {
        Some(true_container) => format!("{}.{}", true_container, module),
        None => module.to_string(),
    }
}

fn _search_for_package_dependency<'a>(
    higher_layer_package: &'a str,
    lower_layer_package: &'a str,
    layers: &'a Vec<&'a str>,
    container: &'a Option<String>,
    graph: &'a ImportGraph,
) -> Option<PackageDependency> {
    let mut temp_graph = graph.clone();
    _remove_other_layers(
        &mut temp_graph,
        layers,
        container,
        (higher_layer_package, lower_layer_package),
    );
    let mut routes: Vec<Route> = vec![];

    // Direct routes.
    let direct_links =
        _pop_direct_imports(higher_layer_package, lower_layer_package, &mut temp_graph);
    for (importer, imported) in direct_links {
        routes.push(Route {
            heads: vec![importer],
            middle: vec![],
            tails: vec![imported],
        });
    }

    // Indirect routes.
    for indirect_route in
        _get_indirect_routes(higher_layer_package, lower_layer_package, &temp_graph)
    {
        routes.push(indirect_route);
    }
    if routes.is_empty() {
        None
    } else {
        Some(PackageDependency {
            imported: graph.ids_by_name[&higher_layer_package],
            importer: graph.ids_by_name[&lower_layer_package],
            routes,
        })
    }
}

fn _layers_from_levels<'a>(levels: &'a Vec<Level>) -> Vec<&'a str> {
    let mut layers: Vec<&str> = vec![];
    for level in levels {
        layers.extend(level.layers.iter());
    }
    layers
}

fn _remove_other_layers<'a>(
    graph: &'a mut ImportGraph,
    layers: &'a Vec<&'a str>,
    container: &'a Option<String>,
    layers_to_preserve: (&'a str, &'a str),
) {
    for layer in layers {
        let layer_module = _module_from_layer(layer, container);
        if layers_to_preserve.0 == layer_module || layers_to_preserve.1 == layer_module {
            continue;
        }
        if graph.contains_module(&layer_module) {
            graph.remove_package(&layer_module);
        }
    }
}

fn _pop_direct_imports<'a>(
    higher_layer_package: &'a str,
    lower_layer_package: &'a str,
    graph: &'a mut ImportGraph,
) -> HashSet<(u32, u32)> {
    // Remove the direct imports, returning them as (importer, imported) tuples.
    let mut imports = HashSet::new();

    let higher_layer_namespace: String = format!("{}.", higher_layer_package);
    let mut lower_layer_module_ids: Vec<u32> = vec![graph.ids_by_name[lower_layer_package]];
    lower_layer_module_ids.append(&mut graph.get_descendant_ids(lower_layer_package));

    for lower_layer_module_id in lower_layer_module_ids {
        let _lower = graph.names_by_id[&lower_layer_module_id];
        let imported_module_ids = graph.importeds_by_importer[&lower_layer_module_id].clone();
        for imported_module_id in imported_module_ids {
            let imported_module = graph.names_by_id[&imported_module_id];

            if imported_module.starts_with(&higher_layer_namespace)
                || imported_module == higher_layer_package
            {
                imports.insert((lower_layer_module_id, imported_module_id));
                graph.remove_import_ids(lower_layer_module_id, imported_module_id)
            }
        }
    }
    imports
}

fn _get_indirect_routes<'a>(
    imported_package: &'a str,
    importer_package: &'a str,
    graph: &'a ImportGraph,
) -> Vec<Route> {
    // Squashes the two packages.
    // Gets a list of paths between them, called middles.
    // Add the heads and tails to the middles.
    let mut temp_graph = graph.clone();
    temp_graph.squash_module(imported_package);
    temp_graph.squash_module(importer_package);

    let middles = _find_middles(&mut temp_graph, importer_package, imported_package);
    _middles_to_routes(graph, middles, importer_package, imported_package)
}

fn _find_middles<'a>(
    graph: &'a mut ImportGraph,
    importer: &'a str,
    imported: &'a str,
) -> Vec<Vec<u32>> {
    let mut middles = vec![];

    for chain in graph.pop_shortest_chains(importer, imported) {
        // Remove first and last element.
        // TODO surely there's a better way?
        let mut middle: Vec<u32> = vec![];
        let chain_length = chain.len();
        for (index, module) in chain.iter().enumerate() {
            if index != 0 && index != chain_length - 1 {
                middle.push(*module);
            }
        }
        middles.push(middle);
    }

    middles
}

fn _log_illegal_route_count(dependency_or_none: &Option<PackageDependency>, duration_in_s: u64) {
    let route_count = match dependency_or_none {
        Some(dependency) => dependency.routes.len(),
        None => 0,
    };
    let pluralized = if route_count == 1 { "" } else { "s" };
    info!(
        "Found {} illegal route{} in {}s.",
        route_count, pluralized, duration_in_s
    );
}

fn _middles_to_routes<'a>(
    graph: &'a ImportGraph,
    middles: Vec<Vec<u32>>,
    importer: &'a str,
    imported: &'a str,
) -> Vec<Route> {
    let mut routes = vec![];
    let importer_id = graph.ids_by_name[&importer];
    let imported_id = graph.ids_by_name[&imported];

    for middle in middles {
        // Construct heads.
        let mut heads: Vec<u32> = vec![];
        let first_imported_id = middle[0];
        let candidate_modules = &graph.importers_by_imported[&first_imported_id];
        for candidate_module in candidate_modules {
            if importer_id == *candidate_module
                || graph
                    .get_descendant_ids(importer)
                    .contains(candidate_module)
            {
                heads.push(*candidate_module);
            }
        }

        // Construct tails.
        let mut tails: Vec<u32> = vec![];
        let last_importer_id = middle[middle.len() - 1];
        let candidate_modules = &graph.importeds_by_importer[&last_importer_id];
        for candidate_module in candidate_modules {
            if imported_id == *candidate_module
                || graph
                    .get_descendant_ids(imported)
                    .contains(candidate_module)
            {
                tails.push(*candidate_module);
            }
        }
        routes.push(Route {
            heads,
            middle,
            tails,
        })
    }

    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_find_illegal_dependencies_no_container() {
        let graph = ImportGraph::new(HashMap::from([
            ("low", HashSet::new()),
            ("low.blue", HashSet::from(["utils"])),
            ("low.green", HashSet::new()),
            ("low.green.alpha", HashSet::from(["high.yellow"])),
            ("mid_a", HashSet::from(["mid_b"])),
            ("mid_a.orange", HashSet::new()),
            ("mid_b", HashSet::from(["mid_c"])),
            ("mid_b.brown", HashSet::new()),
            ("mid_c", HashSet::new()),
            ("mid_c.purple", HashSet::new()),
            ("high", HashSet::from(["low.blue"])),
            ("high.yellow", HashSet::new()),
            ("high.red", HashSet::new()),
            ("high.red.beta", HashSet::new()),
            ("utils", HashSet::from(["high.red"])),
        ]));
        let levels = vec![
            Level {
                layers: vec!["high"],
            },
            Level {
                layers: vec!["mid_a", "mid_b", "mid_c"],
            },
            Level {
                layers: vec!["low"],
            },
        ];
        let containers = HashSet::new();

        let dependencies = find_illegal_dependencies(&graph, &levels, &containers);

        assert_eq!(
            dependencies,
            vec![
                PackageDependency {
                    importer: *graph.ids_by_name.get("low").unwrap(),
                    imported: *graph.ids_by_name.get("high").unwrap(),
                    routes: vec![
                        Route {
                            heads: vec![*graph.ids_by_name.get("low.green.alpha").unwrap()],
                            middle: vec![],
                            tails: vec![*graph.ids_by_name.get("high.yellow").unwrap()],
                        },
                        Route {
                            heads: vec![*graph.ids_by_name.get("low.blue").unwrap()],
                            middle: vec![*graph.ids_by_name.get("utils").unwrap()],
                            tails: vec![*graph.ids_by_name.get("high.red").unwrap()],
                        },
                    ],
                },
                PackageDependency {
                    importer: *graph.ids_by_name.get("mid_a").unwrap(),
                    imported: *graph.ids_by_name.get("mid_b").unwrap(),
                    routes: vec![Route {
                        heads: vec![*graph.ids_by_name.get("mid_a").unwrap()],
                        middle: vec![],
                        tails: vec![*graph.ids_by_name.get("mid_b").unwrap()],
                    },],
                },
                PackageDependency {
                    importer: *graph.ids_by_name.get("mid_b").unwrap(),
                    imported: *graph.ids_by_name.get("mid_c").unwrap(),
                    routes: vec![Route {
                        heads: vec![*graph.ids_by_name.get("mid_b").unwrap()],
                        middle: vec![],
                        tails: vec![*graph.ids_by_name.get("mid_c").unwrap()],
                    },],
                },
            ]
        );
    }

    #[test]
    fn test_find_illegal_dependencies_with_container() {
        let graph = ImportGraph::new(HashMap::from([
            ("mypackage.low", HashSet::new()),
            ("mypackage.low.blue", HashSet::from(["mypackage.utils"])),
            ("mypackage.low.green", HashSet::new()),
            (
                "mypackage.low.green.alpha",
                HashSet::from(["mypackage.high.yellow"]),
            ),
            ("mypackage.high", HashSet::from(["mypackage.low.blue"])),
            ("mypackage.high.yellow", HashSet::new()),
            ("mypackage.high.red", HashSet::new()),
            ("mypackage.high.red.beta", HashSet::new()),
            ("mypackage.utils", HashSet::from(["mypackage.high.red"])),
        ]));
        let levels = vec![
            Level {
                layers: vec!["high"],
            },
            Level {
                layers: vec!["low"],
            },
        ];
        let containers = HashSet::from(["mypackage"]);

        let dependencies = find_illegal_dependencies(&graph, &levels, &containers);

        assert_eq!(
            dependencies,
            vec![PackageDependency {
                importer: *graph.ids_by_name.get("mypackage.low").unwrap(),
                imported: *graph.ids_by_name.get("mypackage.high").unwrap(),
                routes: vec![
                    Route {
                        heads: vec![*graph.ids_by_name.get("mypackage.low.green.alpha").unwrap()],
                        middle: vec![],
                        tails: vec![*graph.ids_by_name.get("mypackage.high.yellow").unwrap()],
                    },
                    Route {
                        heads: vec![*graph.ids_by_name.get("mypackage.low.blue").unwrap()],
                        middle: vec![*graph.ids_by_name.get("mypackage.utils").unwrap()],
                        tails: vec![*graph.ids_by_name.get("mypackage.high.red").unwrap()],
                    },
                ],
            }]
        );
    }

    #[test]
    fn test_generate_module_permutations() {
        let graph = ImportGraph::new(HashMap::from([
            ("mypackage.low", HashSet::new()),
            ("mypackage.low.blue", HashSet::from(["mypackage.utils"])),
            ("mypackage.low.green", HashSet::new()),
            (
                "mypackage.low.green.alpha",
                HashSet::from(["mypackage.high.yellow"]),
            ),
            ("mypackage.mid_a", HashSet::new()),
            ("mypackage.mid_a.foo", HashSet::new()),
            ("mypackage.mid_b", HashSet::new()),
            ("mypackage.mid_b.foo", HashSet::new()),
            ("mypackage.mid_c", HashSet::new()),
            ("mypackage.mid_c.foo", HashSet::new()),
            ("mypackage.high", HashSet::from(["mypackage.low.blue"])),
            ("mypackage.high.yellow", HashSet::new()),
            ("mypackage.high.red", HashSet::new()),
            ("mypackage.high.red.beta", HashSet::new()),
            ("mypackage.utils", HashSet::from(["mypackage.high.red"])),
        ]));
        let levels = vec![
            Level {
                layers: vec!["high"],
            },
            Level {
                layers: vec!["mid_a", "mid_b", "mid_c"],
            },
            Level {
                layers: vec!["low"],
            },
        ];
        let containers = HashSet::from(["mypackage"]);

        let perms = _generate_module_permutations(&graph, &levels, &containers);

        let result: HashSet<(String, String, Option<String>)> = HashSet::from_iter(perms);
        let (high, mid_a, mid_b, mid_c, low) = (
            "mypackage.high",
            "mypackage.mid_a",
            "mypackage.mid_b",
            "mypackage.mid_c",
            "mypackage.low",
        );
        assert_eq!(
            result,
            HashSet::from_iter([
                (
                    high.to_string(),
                    mid_a.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    high.to_string(),
                    mid_b.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    high.to_string(),
                    mid_c.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    high.to_string(),
                    low.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_a.to_string(),
                    mid_b.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_a.to_string(),
                    mid_c.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_b.to_string(),
                    mid_a.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_b.to_string(),
                    mid_c.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_c.to_string(),
                    mid_a.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_c.to_string(),
                    mid_b.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_a.to_string(),
                    low.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_b.to_string(),
                    low.to_string(),
                    Some("mypackage".to_string())
                ),
                (
                    mid_c.to_string(),
                    low.to_string(),
                    Some("mypackage".to_string())
                ),
            ])
        );
    }

    #[test]
    fn test_layers_from_levels() {
        let levels = vec![
            Level {
                layers: vec!["high"],
            },
            Level {
                layers: vec!["medium_a", "medium_b", "medium_c"],
            },
            Level {
                layers: vec!["low"],
            },
        ];

        let result = _layers_from_levels(&levels);

        assert_eq!(
            HashSet::<&str>::from_iter(result),
            HashSet::from_iter(["high", "medium_a", "medium_b", "medium_c", "low",]),
        )
    }
}
