pub mod graph;
pub(crate) mod hierarchy;
pub(crate) mod imports;

use crate::graph::{DetailedImport, Graph};
use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::{PyList, PySet, PyTuple};
use rustc_hash::FxHashSet;

#[pymodule]
fn _rustgrimp(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();

    m.add_class::<GraphWrapper>()?;
    m.add("NoSuchContainer", py.get_type::<NoSuchContainer>())?;
    Ok(())
}

create_exception!(_rustgrimp, NoSuchContainer, pyo3::exceptions::PyException);

#[pyclass(name = "Graph")]
struct GraphWrapper {
    _graph: Graph,
}

/// Wrapper around the Graph struct that integrates with Python.
#[pymethods]
impl GraphWrapper {
    #[new]
    fn new() -> Self {
        GraphWrapper {
            _graph: Graph::default(),
        }
    }

    pub fn get_modules(&self) -> FxHashSet<String> {
        todo!()
    }

    #[pyo3(signature = (module, is_squashed = false))]
    pub fn add_module(&mut self, module: &str, is_squashed: bool) -> PyResult<()> {
        // TODO(peter)
        // if let Some(ancestor_squashed_module) =
        //     self._graph.find_ancestor_squashed_module(&module_struct)
        // {
        //     return Err(PyValueError::new_err(format!(
        //         "Module is a descendant of squashed module {}.",
        //         &ancestor_squashed_module.name
        //     )));
        // }
        //
        // if self._graph.get_modules().contains(&module_struct) {
        //     if self._graph.is_module_squashed(&module_struct) != is_squashed {
        //         return Err(PyValueError::new_err(
        //             "Cannot add a squashed module when it is already present in the graph \
        //             as an unsquashed module, or vice versa.",
        //         ));
        //     }
        // }

        match is_squashed {
            false => self._graph.add_module(module),
            true => self._graph.add_squashed_module(module),
        };
        Ok(())
    }

    pub fn remove_module(&mut self, module: &str) {
        self._graph.remove_module(module);
    }

    pub fn squash_module(&mut self, module: &str) {
        todo!()
    }

    pub fn is_module_squashed(&self, module: &str) -> bool {
        todo!()
    }

    #[pyo3(signature = (*, importer, imported, line_number=None, line_contents=None))]
    pub fn add_import(
        &mut self,
        importer: &str,
        imported: &str,
        line_number: Option<usize>,
        line_contents: Option<&str>,
    ) {
        match (line_number, line_contents) {
            (Some(line_number), Some(line_contents)) => {
                self._graph.add_detailed_import(&DetailedImport {
                    importer,
                    imported,
                    line_number,
                    line_contents,
                });
            }
            (None, None) => {
                self._graph.add_import(&importer, &imported);
            }
            _ => {
                // TODO handle better.
                panic!("Expected line_number and line_contents, or neither.");
            }
        }
    }

    #[pyo3(signature = (*, importer, imported))]
    pub fn remove_import(&mut self, importer: &str, imported: &str) {
        todo!()
    }

    pub fn count_imports(&self) -> usize {
        todo!()
    }

    pub fn find_children(&self, module: &str) -> FxHashSet<String> {
        todo!()
    }

    pub fn find_descendants(&self, module: &str) -> FxHashSet<String> {
        todo!()
    }

    #[pyo3(signature = (*, importer, imported, as_packages = false))]
    pub fn direct_import_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        todo!()
    }

    pub fn find_modules_directly_imported_by(&self, module: &str) -> FxHashSet<String> {
        todo!()
    }

    pub fn find_modules_that_directly_import(&self, module: &str) -> FxHashSet<String> {
        todo!()
    }

    #[pyo3(signature = (*, importer, imported))]
    pub fn get_import_details<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
    ) -> PyResult<Bound<'py, PyList>> {
        todo!()
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_downstream_modules(&self, module: &str, as_package: bool) -> FxHashSet<String> {
        todo!()
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_upstream_modules(&self, module: &str, as_package: bool) -> FxHashSet<String> {
        todo!()
    }

    pub fn find_shortest_chain(&self, importer: &str, imported: &str) -> Option<Vec<String>> {
        todo!()
    }

    #[pyo3(signature = (importer, imported, as_packages=true))]
    pub fn find_shortest_chains<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<Bound<'py, PySet>> {
        todo!()
    }

    #[pyo3(signature = (importer, imported, as_packages=false))]
    pub fn chain_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        todo!()
    }

    #[pyo3(signature = (layers, containers))]
    pub fn find_illegal_dependencies_for_layers<'py>(
        &self,
        py: Python<'py>,
        layers: &Bound<'py, PyTuple>,
        containers: FxHashSet<String>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        todo!()
    }

    pub fn clone(&self) -> GraphWrapper {
        todo!()
    }
}
