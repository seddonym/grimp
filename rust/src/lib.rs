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
use std::collections::HashSet;

#[pymodule]
fn _rustgrimp(_py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();

    m.add_function(wrap_pyfunction!(find_illegal_dependencies, m)?)?;
    m.add("NoSuchContainer", _py.get_type::<NoSuchContainer>())?;
    Ok(())
}

create_exception!(_rustgrimp, NoSuchContainer, pyo3::exceptions::PyException);

#[pyfunction]
pub fn find_illegal_dependencies<'a>(
    py: Python<'a>,
    levels: &'a PyTuple,
    containers: &'a PySet,
    importeds_by_importer: &'a PyDict,
) -> PyResult<&'a PyTuple> {
    info!("Using Rust to find illegal dependencies.");
    let graph = ImportGraph::new(importeds_by_importer.extract()?);

    let levels_rust = rustify_levels(levels);
    let containers_rust: HashSet<&str> = containers.extract()?;

    if let Err(err) = check_containers_exist(&graph, &containers_rust) {
        return Err(NoSuchContainer::new_err(err));
    }

    let dependencies = layers::find_illegal_dependencies(&graph, &levels_rust, &containers_rust);

    convert_dependencies_to_python(py, dependencies, &graph)
}

fn rustify_levels(levels_python: &PyTuple) -> Vec<Level> {
    let mut rust_levels: Vec<Level> = vec![];
    for level_python in levels_python.into_iter() {
        let level_dict = level_python.downcast::<PyDict>().unwrap();
        let layers: HashSet<&str> = level_dict.get_item("layers").unwrap().extract().unwrap();
        let independent: bool = level_dict.get_item("independent").unwrap().extract().unwrap();
        rust_levels.push(Level {
            independent,
            layers: layers.into_iter().collect(),
        });
    }
    rust_levels
}

fn convert_dependencies_to_python<'a>(
    py: Python<'a>,
    dependencies: Vec<PackageDependency>,
    graph: &ImportGraph,
) -> PyResult<&'a PyTuple> {
    let mut python_dependencies: Vec<&PyDict> = vec![];

    for rust_dependency in dependencies {
        let python_dependency = PyDict::new(py);
        python_dependency.set_item("imported", graph.names_by_id[&rust_dependency.imported])?;
        python_dependency.set_item("importer", graph.names_by_id[&rust_dependency.importer])?;
        let mut python_routes: Vec<&PyDict> = vec![];
        for rust_route in rust_dependency.routes {
            let route = PyDict::new(py);
            let heads: Vec<&PyString> = rust_route
                .heads
                .iter()
                .map(|i| PyString::new(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("heads", PyFrozenSet::new(py, &heads)?)?;
            let middle: Vec<&PyString> = rust_route
                .middle
                .iter()
                .map(|i| PyString::new(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("middle", PyTuple::new(py, &middle))?;
            let tails: Vec<&PyString> = rust_route
                .tails
                .iter()
                .map(|i| PyString::new(py, graph.names_by_id[&i]))
                .collect();
            route.set_item("tails", PyFrozenSet::new(py, &tails)?)?;

            python_routes.push(route);
        }

        python_dependency.set_item("routes", PyTuple::new(py, python_routes))?;
        python_dependencies.push(python_dependency)
    }

    Ok(PyTuple::new(py, python_dependencies))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Macro to easily define a python dict.
    // Adapted from the hash_map! macro in https://github.com/jofas/map_macro.
    macro_rules! pydict {
        ($py: ident, {$($k: expr => $v: expr),*, $(,)?}) => {
            {
                let dict = PyDict::new($py);
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
            let elements: Vec<&PyDict> = vec![
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
                })
            ];
            let python_levels: &PyTuple = PyTuple::new(py, elements);

            let result = rustify_levels(python_levels);

            assert_eq!(
                result,
                vec![
                    Level {
                        independent: true,
                        layers: vec!["high"]
                    },
                    Level {
                        independent: true,
                        layers: vec!["medium"]
                    },
                    Level {
                        independent: true,
                        layers: vec!["low"]
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
            let elements: Vec<&PyDict> = vec![
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
                })
            ];
            let python_levels: &PyTuple = PyTuple::new(py, elements);

            let mut result = rustify_levels(python_levels);

            for level in &mut result {
                level.layers.sort();
            }
            assert_eq!(
                result,
                vec![
                    Level {
                        independent: true,
                        layers: vec!["high"]
                    },
                    Level {
                        independent: true,
                        layers: vec!["blue", "green", "orange"]
                    },
                    Level {
                        independent: false,
                        layers: vec!["red", "yellow"]
                    },
                    Level {
                        independent: true,
                        layers: vec!["low"]
                    }
                ]
            );

            Ok(())
        })
        .unwrap();
    }
}
