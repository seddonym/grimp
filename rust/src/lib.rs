pub mod graph;

use crate::graph::{DetailedImport, Graph, Level, Module, PackageDependency};
use log::info;
use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet, PyList, PySet, PyString, PyTuple};
use std::collections::HashSet;

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

    pub fn get_modules(&self) -> HashSet<String> {
        self._graph
            .get_modules()
            .iter()
            .map(|module| module.name.clone())
            .collect()
    }

    #[pyo3(signature = (module, is_squashed = false))]
    pub fn add_module(&mut self, module: &str, is_squashed: bool) -> PyResult<()> {
        let module_struct = Module::new(module.to_string());

        if let Some(ancestor_squashed_module) =
            self._graph.find_ancestor_squashed_module(&module_struct)
        {
            return Err(PyValueError::new_err(format!(
                "Module is a descendant of squashed module {}.",
                &ancestor_squashed_module.name
            )));
        }

        if self._graph.get_modules().contains(&module_struct) {
            if self._graph.is_module_squashed(&module_struct) != is_squashed {
                return Err(PyValueError::new_err(
                    "Cannot add a squashed module when it is already present in the graph \
                    as an unsquashed module, or vice versa.",
                ));
            }
        }

        match is_squashed {
            false => self._graph.add_module(module_struct),
            true => self._graph.add_squashed_module(module_struct),
        };
        Ok(())
    }

    pub fn remove_module(&mut self, module: &str) {
        self._graph.remove_module(&Module::new(module.to_string()));
    }

    pub fn squash_module(&mut self, module: &str) {
        self._graph.squash_module(&Module::new(module.to_string()));
    }

    pub fn is_module_squashed(&self, module: &str) -> bool {
        self._graph
            .is_module_squashed(&Module::new(module.to_string()))
    }

    #[pyo3(signature = (*, importer, imported, line_number=None, line_contents=None))]
    pub fn add_import(
        &mut self,
        importer: &str,
        imported: &str,
        line_number: Option<usize>,
        line_contents: Option<&str>,
    ) {
        let importer = Module::new(importer.to_string());
        let imported = Module::new(imported.to_string());
        match (line_number, line_contents) {
            (Some(line_number), Some(line_contents)) => {
                self._graph.add_detailed_import(&DetailedImport {
                    importer: importer,
                    imported: imported,
                    line_number: line_number,
                    line_contents: line_contents.to_string(),
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
        self._graph.remove_import(
            &Module::new(importer.to_string()),
            &Module::new(imported.to_string()),
        );
    }

    pub fn count_imports(&self) -> usize {
        self._graph.count_imports()
    }

    pub fn find_children(&self, module: &str) -> HashSet<String> {
        self._graph
            .find_children(&Module::new(module.to_string()))
            .iter()
            .map(|child| child.name.clone())
            .collect()
    }

    pub fn find_descendants(&self, module: &str) -> HashSet<String> {
        self._graph
            .find_descendants(&Module::new(module.to_string()))
            .unwrap()
            .iter()
            .map(|descendant| descendant.name.clone())
            .collect()
    }

    #[pyo3(signature = (*, importer, imported, as_packages = false))]
    pub fn direct_import_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        if as_packages {
            let importer_module = Module::new(importer.to_string());
            let imported_module = Module::new(imported.to_string());
            // Raise a ValueError if they are in the same package.
            // (direct_import_exists) will panic if they are passed.
            // TODO - this is a simpler check than Python, is it enough?
            if importer_module.is_descendant_of(&imported_module)
                || imported_module.is_descendant_of(&importer_module)
            {
                return Err(PyValueError::new_err("Modules have shared descendants."));
            }
        }

        Ok(self._graph.direct_import_exists(
            &Module::new(importer.to_string()),
            &Module::new(imported.to_string()),
            as_packages,
        ))
    }

    pub fn find_modules_directly_imported_by(&self, module: &str) -> HashSet<String> {
        self._graph
            .find_modules_directly_imported_by(&Module::new(module.to_string()))
            .iter()
            .map(|imported| imported.name.clone())
            .collect()
    }

    pub fn find_modules_that_directly_import(&self, module: &str) -> HashSet<String> {
        self._graph
            .find_modules_that_directly_import(&Module::new(module.to_string()))
            .iter()
            .map(|importer| importer.name.clone())
            .collect()
    }

    #[pyo3(signature = (*, importer, imported))]
    pub fn get_import_details<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
    ) -> PyResult<Bound<'py, PyList>> {
        let mut vector: Vec<Bound<PyDict>> = vec![];

        let mut rust_import_details_vec: Vec<DetailedImport> = self
            ._graph
            .get_import_details(
                &Module::new(importer.to_string()),
                &Module::new(imported.to_string()),
            )
            .into_iter()
            .collect();
        rust_import_details_vec.sort();

        for detailed_import in rust_import_details_vec {
            let pydict = PyDict::new(py);
            pydict.set_item(
                "importer".to_string(),
                detailed_import.importer.name.clone(),
            )?;
            pydict.set_item(
                "imported".to_string(),
                detailed_import.imported.name.clone(),
            )?;
            pydict.set_item("line_number".to_string(), detailed_import.line_number)?;
            pydict.set_item(
                "line_contents".to_string(),
                detailed_import.line_contents.clone(),
            )?;
            vector.push(pydict);
        }
        PyList::new(py, &vector)
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_downstream_modules(&self, module: &str, as_package: bool) -> HashSet<String> {
        // Turn the Modules to Strings.
        self._graph
            .find_downstream_modules(&Module::new(module.to_string()), as_package)
            .iter()
            .map(|downstream| downstream.name.clone())
            .collect()
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_upstream_modules(&self, module: &str, as_package: bool) -> HashSet<String> {
        self._graph
            .find_upstream_modules(&Module::new(module.to_string()), as_package)
            .iter()
            .map(|upstream| upstream.name.clone())
            .collect()
    }

    pub fn find_shortest_chain(&self, importer: &str, imported: &str) -> Option<Vec<String>> {
        let chain = self._graph.find_shortest_chain(
            &Module::new(importer.to_string()),
            &Module::new(imported.to_string()),
        )?;

        Some(chain.iter().map(|module| module.name.clone()).collect())
    }

    #[pyo3(signature = (importer, imported, as_packages=true))]
    pub fn find_shortest_chains<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<Bound<'py, PySet>> {
        let rust_chains: HashSet<Vec<Module>> = self
            ._graph
            .find_shortest_chains(
                &Module::new(importer.to_string()),
                &Module::new(imported.to_string()),
                as_packages,
            )
            .map_err(|string| PyValueError::new_err(string))?;

        let mut tuple_chains: Vec<Bound<'py, PyTuple>> = vec![];
        for rust_chain in rust_chains.iter() {
            let module_names: Vec<Bound<'py, PyString>> = rust_chain
                .iter()
                .map(|module| PyString::new(py, &module.name))
                .collect();
            let tuple = PyTuple::new(py, &module_names)?;
            tuple_chains.push(tuple);
        }
        PySet::new(py, &tuple_chains)
    }

    #[pyo3(signature = (importer, imported, as_packages=false))]
    pub fn chain_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        if as_packages {
            let importer_module = Module::new(importer.to_string());
            let imported_module = Module::new(imported.to_string());
            // Raise a ValueError if they are in the same package.
            // TODO - this is a simpler check than Python, is it enough?
            if importer_module.is_descendant_of(&imported_module)
                || imported_module.is_descendant_of(&importer_module)
            {
                return Err(PyValueError::new_err("Modules have shared descendants."));
            }
        }
        Ok(self._graph.chain_exists(
            &Module::new(importer.to_string()),
            &Module::new(imported.to_string()),
            as_packages,
        ))
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (layers, containers))]
    pub fn find_illegal_dependencies_for_layers<'py>(
        &self,
        py: Python<'py>,
        layers: &Bound<'py, PyTuple>,
        containers: HashSet<String>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        info!("Using Rust to find illegal dependencies.");
        let levels = rustify_levels(layers);

        println!("\nIncoming {:?}, {:?}", levels, containers);
        let dependencies = py.allow_threads(|| {
            self._graph
                .find_illegal_dependencies_for_layers(levels, containers)
        });
        match dependencies {
            Ok(dependencies) => _convert_dependencies_to_python(py, &dependencies),
            Err(error) => Err(NoSuchContainer::new_err(format!(
                "Container {} does not exist.",
                error.container
            ))),
        }
    }
    pub fn clone(&self) -> GraphWrapper {
        GraphWrapper {
            _graph: self._graph.clone(),
        }
    }
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

fn _convert_dependencies_to_python<'py>(
    py: Python<'py>,
    dependencies: &Vec<PackageDependency>,
) -> PyResult<Bound<'py, PyTuple>> {
    let mut python_dependencies: Vec<Bound<'py, PyDict>> = vec![];

    for rust_dependency in dependencies {
        let python_dependency = PyDict::new(py);
        python_dependency.set_item("imported", &rust_dependency.imported.name)?;
        python_dependency.set_item("importer", &rust_dependency.importer.name)?;
        let mut python_routes: Vec<Bound<'py, PyDict>> = vec![];
        for rust_route in &rust_dependency.routes {
            let route = PyDict::new(py);
            let heads: Vec<Bound<'py, PyString>> = rust_route
                .heads
                .iter()
                .map(|module| PyString::new(py, &module.name))
                .collect();
            route.set_item("heads", PyFrozenSet::new(py, &heads)?)?;
            let middle: Vec<Bound<'py, PyString>> = rust_route
                .middle
                .iter()
                .map(|module| PyString::new(py, &module.name))
                .collect();
            route.set_item("middle", PyTuple::new(py, &middle)?)?;
            let tails: Vec<Bound<'py, PyString>> = rust_route
                .tails
                .iter()
                .map(|module| PyString::new(py, &module.name))
                .collect();
            route.set_item("tails", PyFrozenSet::new(py, &tails)?)?;

            python_routes.push(route);
        }

        python_dependency.set_item("routes", PyTuple::new(py, python_routes)?)?;
        python_dependencies.push(python_dependency)
    }

    PyTuple::new(py, python_dependencies)
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
            let python_levels = PyTuple::new(py, elements)?;

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
            let python_levels = PyTuple::new(py, elements)?;

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
