use pyo3::prelude::*;
use std::path::PathBuf;

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
#[pyo3(signature = (package, include_external_packages=false, exclude_type_checking_imports=false, cache_dir=None))]
pub fn build_graph_rust(
    package: &PyPackageSpec,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Option<String>,
) -> GraphWrapper {
    let cache_path = cache_dir.map(PathBuf::from);
    let graph = build_graph(
        &package.inner,
        include_external_packages,
        exclude_type_checking_imports,
        cache_path.as_ref(),
    );
    GraphWrapper::from_graph(graph)
}
