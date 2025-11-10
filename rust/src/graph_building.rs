use pyo3::prelude::*;
use std::path::PathBuf;

use crate::graph::GraphWrapper;
use crate::graph::builder::{GraphBuilder, PackageSpec};

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

#[pyclass(name = "GraphBuilder")]
pub struct PyGraphBuilder {
    inner: GraphBuilder,
}

#[pymethods]
impl PyGraphBuilder {
    #[new]
    fn new(package: PyPackageSpec) -> Self {
        PyGraphBuilder {
            inner: GraphBuilder::new(package.inner),
        }
    }

    fn include_external_packages(mut self_: PyRefMut<'_, Self>, yes: bool) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().include_external_packages(yes);
        self_
    }

    fn exclude_type_checking_imports(
        mut self_: PyRefMut<'_, Self>,
        yes: bool,
    ) -> PyRefMut<'_, Self> {
        self_.inner = self_.inner.clone().exclude_type_checking_imports(yes);
        self_
    }

    fn build(&self) -> GraphWrapper {
        let graph = self.inner.build();
        GraphWrapper::from_graph(graph)
    }
}
