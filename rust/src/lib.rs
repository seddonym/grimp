mod containers;
// TODO make these private.
pub mod dependencies;
pub mod importgraph;
pub mod layers;

use crate::dependencies::PackageDependency;
use containers::check_containers_exist;
use importgraph::ImportGraph;
use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet, PySet, PyString, PyTuple};
use std::collections::HashSet;
use log::info;


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
    layers: &'a PyTuple,
    containers: &'a PySet,
    importeds_by_importer: &'a PyDict,
) -> PyResult<&'a PyTuple> {
    info!("Using Rust to find illegal dependencies.");
    let graph = ImportGraph::new(importeds_by_importer.extract()?);
    let layers_rust: Vec<&str> = layers.extract()?;
    let containers_rust: HashSet<&str> = containers.extract()?;

    if let Err(err) = check_containers_exist(&graph, &containers_rust) {
        return Err(NoSuchContainer::new_err(err));
    }

    let dependencies = layers::find_illegal_dependencies(&graph, &layers_rust, &containers_rust);

    convert_dependencies_to_python(py, dependencies, &graph)
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
