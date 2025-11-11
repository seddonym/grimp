use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::path::PathBuf;

use crate::errors::GrimpError;
use crate::graph::GraphWrapper;
use crate::graph::builder::{PackageSpec, build_graph};

#[pyclass(name = "PackageSpec")]
#[derive(Clone)]
pub struct PyPackageSpec {
    inner: PackageSpec,
}

#[pymethods]
impl PyPackageSpec {
    #[new]
    fn new(name: String, directory: String) -> Self {
        PyPackageSpec {
            inner: PackageSpec::new(name, PathBuf::from(directory)),
        }
    }
}

#[pyfunction]
#[pyo3(signature = (packages, include_external_packages=false, exclude_type_checking_imports=false, cache_dir=None))]
pub fn build_graph_rust(
    py: Python,
    packages: Vec<PyPackageSpec>,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Option<String>,
) -> PyResult<GraphWrapper> {
    let cache_path = cache_dir.map(PathBuf::from);

    // Extract the inner PackageSpec from each PyPackageSpec
    let package_specs: Vec<PackageSpec> = packages.iter().map(|p| p.inner.clone()).collect();

    let graph_result = build_graph(
        &package_specs,
        include_external_packages,
        exclude_type_checking_imports,
        cache_path.as_ref(),
    );

    match graph_result {
        Ok(graph) => Ok(GraphWrapper::from_graph(graph)),
        Err(GrimpError::ParseError {
            module_filename,
            line_number,
            text,
            ..
        }) => {
            // Import the Python SourceSyntaxError from grimp.exceptions
            let exceptions_module = PyModule::import(py, "grimp.exceptions")?;
            let source_syntax_error = exceptions_module.getattr("SourceSyntaxError")?;
            let exception = source_syntax_error.call1((module_filename, line_number, text))?;
            Err(PyErr::from_value(exception))
        }
        Err(e) => Err(e.into()),
    }
}
