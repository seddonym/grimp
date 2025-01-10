mod containers;
// TODO make these private.
pub mod dependencies;
pub mod importgraph;
pub mod layers;

use crate::dependencies::PackageDependency;
use containers::check_containers_exist;
use importgraph::ImportGraph;
use layers::Level;
use log::info;
use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet, PySet, PyString, PyTuple};
use std::collections::{HashMap, HashSet};
pub mod graph;

#[pymodule]
fn _rustgrimp(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    m.add_function(wrap_pyfunction!(find_illegal_dependencies, m)?)?;
    m.add("NoSuchContainer", py.get_type_bound::<NoSuchContainer>())?;
    Ok(())
}

create_exception!(_rustgrimp, NoSuchContainer, pyo3::exceptions::PyException);

#[pyfunction]
pub fn find_illegal_dependencies<'py>(
    py: Python<'py>,
    levels: &Bound<'py, PyTuple>,
    containers: &Bound<'py, PySet>,
    importeds_by_importer: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyTuple>> {
    info!("Using Rust to find illegal dependencies.");

    let importeds_by_importer_strings: HashMap<String, HashSet<String>> =
        importeds_by_importer.extract()?;
    let importeds_by_importer_strs = strings_to_strs_hashmap(&importeds_by_importer_strings);

    let graph = ImportGraph::new(importeds_by_importer_strs);
    let levels_rust = rustify_levels(levels);
    let containers_rust: HashSet<String> = containers.extract()?;

    if let Err(err) = check_containers_exist(&graph, &containers_rust) {
        return Err(NoSuchContainer::new_err(err));
    }

    let dependencies = py.allow_threads(|| {
        layers::find_illegal_dependencies(&graph, &levels_rust, &containers_rust)
    });

    convert_dependencies_to_python(py, dependencies, &graph)
}

fn strings_to_strs_hashmap<'a>(
    string_map: &'a HashMap<String, HashSet<String>>,
) -> HashMap<&'a str, HashSet<&'a str>> {
    let mut str_map: HashMap<&str, HashSet<&str>> = HashMap::new();

    for (key, set) in string_map {
        let mut str_set: HashSet<&str> = HashSet::new();
        for item in set.iter() {
            str_set.insert(item);
        }
        str_map.insert(key.as_str(), str_set);
    }
    str_map
}

fn rustify_levels<'a>(levels_python: &Bound<'a, PyTuple>) -> Vec<Level> {
    let mut rust_levels: Vec<Level> = vec![];
    for level_python in levels_python.into_iter() {
        let level_dict = level_python.downcast::<PyDict>().unwrap();
        let layers: HashSet<String> = level_dict
            .get_item("layers")
            .unwrap()
            .unwrap()
            .extract()
            .unwrap();

        let independent: bool = level_dict
            .get_item("independent")
            .unwrap()
            .unwrap()
            .extract()
            .unwrap();
        rust_levels.push(Level {
            independent,
            layers: layers.into_iter().collect(),
        });
    }
    rust_levels
}

fn convert_dependencies_to_python<'py>(
    py: Python<'py>,
    dependencies: Vec<PackageDependency>,
    graph: &ImportGraph,
) -> PyResult<Bound<'py, PyTuple>> {
    let mut python_dependencies: Vec<Bound<'py, PyDict>> = vec![];

    for rust_dependency in dependencies {
        let python_dependency = PyDict::new_bound(py);
        python_dependency.set_item("imported", graph.names_by_id[&rust_dependency.imported])?;
        python_dependency.set_item("importer", graph.names_by_id[&rust_dependency.importer])?;
        let mut python_routes: Vec<Bound<'py, PyDict>> = vec![];
        for rust_route in rust_dependency.routes {
            let route = PyDict::new_bound(py);
            let heads: Vec<Bound<'py, PyString>> = rust_route
                .heads
                .iter()
                .map(|i| PyString::new_bound(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("heads", PyFrozenSet::new_bound(py, &heads)?)?;
            let middle: Vec<Bound<'py, PyString>> = rust_route
                .middle
                .iter()
                .map(|i| PyString::new_bound(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("middle", PyTuple::new_bound(py, &middle))?;
            let tails: Vec<Bound<'py, PyString>> = rust_route
                .tails
                .iter()
                .map(|i| PyString::new_bound(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("tails", PyFrozenSet::new_bound(py, &tails)?)?;

            python_routes.push(route);
        }

        python_dependency.set_item("routes", PyTuple::new_bound(py, python_routes))?;
        python_dependencies.push(python_dependency)
    }

    Ok(PyTuple::new_bound(py, python_dependencies))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Macro to easily define a python dict.
    // Adapted from the hash_map! macro in https://github.com/jofas/map_macro.
    macro_rules! pydict {
        ($py: ident, {$($k: expr => $v: expr),*, $(,)?}) => {
            {
                let dict = PyDict::new_bound($py);
                $(
                    dict.set_item($k, $v)?;
                )*
                dict
            }
        };
    }

    #[test]
    fn test_rustify_levels_no_sibling_layers() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| -> PyResult<()> {
            let elements = vec![
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["high"]),
                }),
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["medium"]),
                }),
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["low"]),
                }),
            ];
            let python_levels = PyTuple::new_bound(py, elements);

            let result = rustify_levels(&python_levels);

            assert_eq!(
                result,
                vec![
                    Level {
                        independent: true,
                        layers: vec!["high".to_string()]
                    },
                    Level {
                        independent: true,
                        layers: vec!["medium".to_string()]
                    },
                    Level {
                        independent: true,
                        layers: vec!["low".to_string()]
                    }
                ]
            );

            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_rustify_levels_sibling_layers() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| -> PyResult<()> {
            let elements = vec![
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["high"]),
                }),
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["blue", "green", "orange"]),
                }),
                pydict! (py, {
                    "independent" => false,
                    "layers" => HashSet::from(["red", "yellow"]),
                }),
                pydict! (py, {
                    "independent" => true,
                    "layers" => HashSet::from(["low"]),
                }),
            ];
            let python_levels = PyTuple::new_bound(py, elements);

            let mut result = rustify_levels(&python_levels);

            for level in &mut result {
                level.layers.sort();
            }
            assert_eq!(
                result,
                vec![
                    Level {
                        independent: true,
                        layers: vec!["high".to_string()]
                    },
                    Level {
                        independent: true,
                        layers: vec![
                            "blue".to_string(),
                            "green".to_string(),
                            "orange".to_string()
                        ]
                    },
                    Level {
                        independent: false,
                        layers: vec!["red".to_string(), "yellow".to_string()]
                    },
                    Level {
                        independent: true,
                        layers: vec!["low".to_string()]
                    }
                ]
            );

            Ok(())
        })
        .unwrap();
    }
}
