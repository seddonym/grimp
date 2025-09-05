use bimap::BiMap;
use derive_new::new;
use getset::{CopyGetters, Getters};
use itertools::Itertools;
use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict, PyFrozenSet, PyList, PySet, PyString, PyTuple};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};
use string_interner::backend::StringBackend;
use string_interner::{DefaultSymbol, StringInterner};

use crate::errors::{GrimpError, GrimpResult, ModuleNotPresent};
use crate::graph::higher_order_queries::Level;
use crate::graph::higher_order_queries::PackageDependency as PyPackageDependency;
use crate::module_expressions::ModuleExpression;

pub mod direct_import_queries;
pub mod graph_manipulation;
pub mod hierarchy_queries;
pub mod higher_order_queries;
pub mod import_chain_queries;

pub(crate) mod pathfinding;

static MODULE_NAMES: LazyLock<RwLock<StringInterner<StringBackend>>> =
    LazyLock::new(|| RwLock::new(StringInterner::default()));
static IMPORT_LINE_CONTENTS: LazyLock<RwLock<StringInterner<StringBackend>>> =
    LazyLock::new(|| RwLock::new(StringInterner::default()));
static EMPTY_MODULE_TOKENS: LazyLock<FxHashSet<ModuleToken>> = LazyLock::new(FxHashSet::default);
static EMPTY_IMPORT_DETAILS: LazyLock<FxHashSet<PyImportDetails>> =
    LazyLock::new(FxHashSet::default);

new_key_type! { pub struct ModuleToken; }

#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters, CopyGetters)]
pub struct Module {
    #[getset(get_copy = "pub")]
    token: ModuleToken,

    #[getset(get_copy = "pub")]
    interned_name: DefaultSymbol,

    // Invisible modules exist in the hierarchy but haven't been explicitly added to the graph.
    #[getset(get_copy = "pub")]
    is_invisible: bool,

    #[getset(get_copy = "pub")]
    is_squashed: bool,
}

impl Module {
    pub fn name(&self) -> String {
        let interner = MODULE_NAMES.read().unwrap();
        interner.resolve(self.interned_name).unwrap().to_owned()
    }
}

#[derive(Default, Clone)]
pub struct Graph {
    // Hierarchy
    modules_by_name: BiMap<DefaultSymbol, ModuleToken>,
    modules: SlotMap<ModuleToken, Module>,
    module_parents: SecondaryMap<ModuleToken, Option<ModuleToken>>,
    module_children: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    // Imports
    imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    reverse_imports: SecondaryMap<ModuleToken, FxHashSet<ModuleToken>>,
    import_details: FxHashMap<(ModuleToken, ModuleToken), FxHashSet<PyImportDetails>>,
}

#[pyclass(name = "Graph")]
#[derive(Clone)]
pub struct GraphWrapper {
    _graph: Graph,
}

impl GraphWrapper {
    fn get_visible_module_by_name(&self, name: &str) -> Result<&Module, ModuleNotPresent> {
        self._graph
            .get_module_by_name(name)
            .filter(|m| !m.is_invisible())
            .ok_or(ModuleNotPresent(name.to_owned()))
    }

    fn parse_containers(
        &self,
        containers: &HashSet<String>,
    ) -> Result<HashSet<&Module>, GrimpError> {
        containers
            .iter()
            .map(|name| match self.get_visible_module_by_name(name) {
                Ok(module) => Ok(module),
                Err(ModuleNotPresent(_)) => Err(GrimpError::NoSuchContainer(name.into()))?,
            })
            .collect::<Result<HashSet<_>, GrimpError>>()
    }

    fn parse_levels_by_container(
        &self,
        pylevels: &Bound<'_, PyTuple>,
        containers: &HashSet<&Module>,
    ) -> Vec<Vec<Level>> {
        let containers = match containers.is_empty() {
            true => vec![None],
            false => containers.iter().map(|c| Some(c.name())).collect(),
        };

        let mut levels_by_container: Vec<Vec<Level>> = vec![];
        for container in containers {
            let mut levels: Vec<Level> = vec![];
            for pylevel in pylevels.into_iter() {
                let level_dict = pylevel.downcast::<PyDict>().unwrap();
                let layers = level_dict
                    .get_item("layers")
                    .unwrap()
                    .unwrap()
                    .extract::<HashSet<String>>()
                    .unwrap()
                    .into_iter()
                    .map(|name| match container.clone() {
                        Some(container) => format!("{container}.{name}"),
                        None => name,
                    })
                    .filter_map(|name| match self.get_visible_module_by_name(&name) {
                        Ok(module) => Some(module.token()),
                        // TODO(peter) Error here? Or silently continue (backwards compatibility?)
                        Err(ModuleNotPresent(_)) => None,
                    })
                    .collect::<FxHashSet<_>>();

                let independent = level_dict
                    .get_item("independent")
                    .unwrap()
                    .unwrap()
                    .extract::<bool>()
                    .unwrap();

                let closed = level_dict
                    .get_item("closed")
                    .unwrap()
                    .unwrap()
                    .extract::<bool>()
                    .unwrap();

                levels.push(Level::new(layers, independent, closed));
            }
            levels_by_container.push(levels);
        }

        levels_by_container
    }

    fn convert_package_dependencies_to_python<'py>(
        &self,
        py: Python<'py>,
        package_dependencies: Vec<PackageDependency>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        let mut python_dependencies: Vec<Bound<'py, PyDict>> = vec![];

        for rust_dependency in package_dependencies {
            let python_dependency = PyDict::new(py);
            python_dependency.set_item("imported", &rust_dependency.imported)?;
            python_dependency.set_item("importer", &rust_dependency.importer)?;
            let mut python_routes: Vec<Bound<'py, PyDict>> = vec![];
            for rust_route in &rust_dependency.routes {
                let route = PyDict::new(py);
                let heads: Vec<Bound<'py, PyString>> = rust_route
                    .heads
                    .iter()
                    .map(|module| PyString::new(py, module))
                    .collect();
                route.set_item("heads", PyFrozenSet::new(py, &heads)?)?;
                let middle: Vec<Bound<'py, PyString>> = rust_route
                    .middle
                    .iter()
                    .map(|module| PyString::new(py, module))
                    .collect();
                route.set_item("middle", PyTuple::new(py, &middle)?)?;
                let tails: Vec<Bound<'py, PyString>> = rust_route
                    .tails
                    .iter()
                    .map(|module| PyString::new(py, module))
                    .collect();
                route.set_item("tails", PyFrozenSet::new(py, &tails)?)?;

                python_routes.push(route);
            }

            python_dependency.set_item("routes", PyTuple::new(py, python_routes)?)?;
            python_dependencies.push(python_dependency)
        }

        PyTuple::new(py, python_dependencies)
    }
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
        self._graph.all_modules().visible().names().collect()
    }

    pub fn contains_module(&self, name: &str) -> bool {
        self.get_visible_module_by_name(name).is_ok()
    }

    #[pyo3(signature = (module, is_squashed = false))]
    pub fn add_module(&mut self, module: &str, is_squashed: bool) -> PyResult<()> {
        for ancestor_module in self
            ._graph
            .module_name_to_self_and_ancestors(module)
            .into_iter()
            .skip(1)
        {
            if self.is_module_squashed(&ancestor_module).unwrap_or(false) {
                return Err(PyValueError::new_err(format!(
                    "Module is a descendant of squashed module {}.",
                    &ancestor_module,
                )));
            };
        }

        if self.contains_module(module) && self.is_module_squashed(module)? != is_squashed {
            return Err(PyValueError::new_err(
                "Cannot add a squashed module when it is already present in the graph \
                        as an unsquashed module, or vice versa.",
            ));
        }

        match is_squashed {
            false => self._graph.get_or_add_module(module),
            true => self._graph.get_or_add_squashed_module(module),
        };
        Ok(())
    }

    pub fn remove_module(&mut self, module: &str) {
        if let Some(module) = self._graph.get_module_by_name(module) {
            self._graph.remove_module(module.token())
        }
    }

    pub fn squash_module(&mut self, module: &str) -> PyResult<()> {
        let module = self.get_visible_module_by_name(module)?.token();
        self._graph.squash_module(module);
        Ok(())
    }

    pub fn is_module_squashed(&self, module: &str) -> PyResult<bool> {
        Ok(self.get_visible_module_by_name(module)?.is_squashed())
    }

    #[pyo3(signature = (*, importer, imported, line_number=None, line_contents=None))]
    pub fn add_import(
        &mut self,
        importer: &str,
        imported: &str,
        line_number: Option<u32>,
        line_contents: Option<&str>,
    ) {
        let importer = self._graph.get_or_add_module(importer).token();
        let imported = self._graph.get_or_add_module(imported).token();
        match (line_number, line_contents) {
            (Some(line_number), Some(line_contents)) => {
                self._graph
                    .add_detailed_import(importer, imported, line_number, line_contents)
            }
            (None, None) => {
                self._graph.add_import(importer, imported);
            }
            _ => {
                // TODO handle better.
                panic!("Expected line_number and line_contents, or neither.");
            }
        }
    }

    #[pyo3(signature = (*, importer, imported))]
    pub fn remove_import(&mut self, importer: &str, imported: &str) -> PyResult<()> {
        let importer = self.get_visible_module_by_name(importer)?.token();
        let imported = self.get_visible_module_by_name(imported)?.token();
        self._graph.remove_import(importer, imported);
        Ok(())
    }

    pub fn count_imports(&self) -> usize {
        self._graph.count_imports()
    }

    pub fn find_children(&self, module: &str) -> PyResult<HashSet<String>> {
        let module = self
            ._graph
            .get_module_by_name(module)
            .ok_or(ModuleNotPresent(module.to_owned()))?;
        Ok(self
            ._graph
            .get_module_children(module.token())
            .visible()
            .names()
            .collect())
    }

    pub fn find_descendants(&self, module: &str) -> PyResult<HashSet<String>> {
        let module = self
            ._graph
            .get_module_by_name(module)
            .ok_or(ModuleNotPresent(module.to_owned()))?;
        Ok(self
            ._graph
            .get_module_descendants(module.token())
            .visible()
            .names()
            .collect())
    }

    pub fn find_matching_modules(&self, expression: &str) -> PyResult<HashSet<String>> {
        let expression: ModuleExpression = expression.parse()?;
        Ok(self
            ._graph
            .find_matching_modules(&expression)
            .visible()
            .names()
            .collect())
    }

    #[pyo3(signature = (*, importer, imported, as_packages = false))]
    pub fn direct_import_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        let importer = self.get_visible_module_by_name(importer)?.token();
        let imported = self.get_visible_module_by_name(imported)?.token();
        Ok(self
            ._graph
            .direct_import_exists(importer, imported, as_packages)?)
    }

    pub fn find_modules_directly_imported_by(&self, module: &str) -> PyResult<HashSet<String>> {
        let module = self.get_visible_module_by_name(module)?.token();
        Ok(self
            ._graph
            .modules_directly_imported_by(module)
            .iter()
            .into_module_iterator(&self._graph)
            .visible()
            .names()
            .collect())
    }

    pub fn find_modules_that_directly_import(&self, module: &str) -> PyResult<HashSet<String>> {
        let module = self.get_visible_module_by_name(module)?.token();
        Ok(self
            ._graph
            .modules_that_directly_import(module)
            .iter()
            .into_module_iterator(&self._graph)
            .visible()
            .names()
            .collect())
    }

    #[pyo3(signature = (*, importer, imported))]
    pub fn get_import_details<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
    ) -> PyResult<Bound<'py, PyList>> {
        let importer = match self._graph.get_module_by_name(importer) {
            Some(module) => module,
            None => return Ok(PyList::empty(py)),
        };
        let imported = match self._graph.get_module_by_name(imported) {
            Some(module) => module,
            None => return Ok(PyList::empty(py)),
        };

        PyList::new(
            py,
            self._graph
                .get_import_details(importer.token(), imported.token())
                .iter()
                .map(|import_details| {
                    ImportDetails::new(
                        importer.name(),
                        imported.name(),
                        import_details.line_number(),
                        import_details.line_contents(),
                    )
                })
                .sorted()
                .map(|import_details| {
                    [
                        ("importer", import_details.importer.into_py_any(py).unwrap()),
                        ("imported", import_details.imported.into_py_any(py).unwrap()),
                        (
                            "line_number",
                            import_details.line_number.into_py_any(py).unwrap(),
                        ),
                        (
                            "line_contents",
                            import_details.line_contents.into_py_any(py).unwrap(),
                        ),
                    ]
                    .into_py_dict(py)
                    .unwrap()
                }),
        )
    }

    #[pyo3(signature = (*, importer_expression, imported_expression))]
    pub fn find_matching_direct_imports<'py>(
        &self,
        py: Python<'py>,
        importer_expression: &str,
        imported_expression: &str,
    ) -> PyResult<Bound<'py, PyList>> {
        let importer_expression: ModuleExpression = importer_expression.parse()?;
        let imported_expression: ModuleExpression = imported_expression.parse()?;

        let matching_imports = self
            ._graph
            .find_matching_direct_imports(&importer_expression, &imported_expression);

        PyList::new(
            py,
            matching_imports
                .into_iter()
                .map(|(importer, imported)| {
                    let importer = self._graph.get_module(importer).unwrap();
                    let imported = self._graph.get_module(imported).unwrap();
                    Import::new(importer.name(), imported.name())
                })
                .sorted()
                .map(|import| {
                    [
                        ("importer", import.importer.into_py_any(py).unwrap()),
                        ("imported", import.imported.into_py_any(py).unwrap()),
                    ]
                    .into_py_dict(py)
                    .unwrap()
                }),
        )
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_downstream_modules(
        &self,
        module: &str,
        as_package: bool,
    ) -> PyResult<HashSet<String>> {
        let module = self.get_visible_module_by_name(module)?.token();
        Ok(self
            ._graph
            .find_downstream_modules(module, as_package)
            .iter()
            .into_module_iterator(&self._graph)
            .visible()
            .names()
            .collect())
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (module, as_package=false))]
    pub fn find_upstream_modules(
        &self,
        module: &str,
        as_package: bool,
    ) -> PyResult<HashSet<String>> {
        let module = self.get_visible_module_by_name(module)?.token();
        Ok(self
            ._graph
            .find_upstream_modules(module, as_package)
            .iter()
            .into_module_iterator(&self._graph)
            .visible()
            .names()
            .collect())
    }

    #[pyo3(signature = (importer, imported, as_packages=false))]
    pub fn find_shortest_chain(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<Option<Vec<String>>> {
        let importer = self.get_visible_module_by_name(importer)?.token();
        let imported = self.get_visible_module_by_name(imported)?.token();
        Ok(self
            ._graph
            .find_shortest_chain(importer, imported, as_packages)?
            .map(|chain| {
                chain
                    .iter()
                    .into_module_iterator(&self._graph)
                    .names()
                    .collect()
            }))
    }

    #[pyo3(signature = (importer, imported, as_packages=false))]
    pub fn chain_exists(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<bool> {
        let importer = self.get_visible_module_by_name(importer)?.token();
        let imported = self.get_visible_module_by_name(imported)?.token();
        Ok(self._graph.chain_exists(importer, imported, as_packages)?)
    }

    #[pyo3(signature = (importer, imported, as_packages=true))]
    pub fn find_shortest_chains<'py>(
        &self,
        py: Python<'py>,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> PyResult<Bound<'py, PySet>> {
        let importer = self.get_visible_module_by_name(importer)?.token();
        let imported = self.get_visible_module_by_name(imported)?.token();
        let chains = self
            ._graph
            .find_shortest_chains(importer, imported, as_packages)?
            .into_iter()
            .map(|chain| {
                PyTuple::new(
                    py,
                    chain
                        .iter()
                        .into_module_iterator(&self._graph)
                        .names()
                        .collect::<Vec<_>>(),
                )
                .unwrap()
            });
        PySet::new(py, chains)
    }

    #[pyo3(signature = (layers, containers))]
    pub fn find_illegal_dependencies_for_layers<'py>(
        &self,
        py: Python<'py>,
        layers: &Bound<'py, PyTuple>,
        containers: HashSet<String>,
    ) -> PyResult<Bound<'py, PyTuple>> {
        let containers = self.parse_containers(&containers)?;
        let levels_by_container = self.parse_levels_by_container(layers, &containers);

        let illegal_dependencies = levels_by_container
            .into_iter()
            .par_bridge()
            .try_fold(
                Vec::new,
                |mut v: Vec<PyPackageDependency>, levels| -> GrimpResult<_> {
                    v.extend(self._graph.find_illegal_dependencies_for_layers(&levels)?);
                    Ok(v)
                },
            )
            .try_reduce(
                Vec::new,
                |mut v: Vec<PyPackageDependency>, package_dependencies| {
                    v.extend(package_dependencies);
                    Ok(v)
                },
            )?;

        let illegal_dependencies = illegal_dependencies
            .into_iter()
            .map(|dep| {
                PackageDependency::new(
                    self._graph.get_module(*dep.importer()).unwrap().name(),
                    self._graph.get_module(*dep.imported()).unwrap().name(),
                    dep.routes()
                        .iter()
                        .map(|route| {
                            Route::new(
                                route
                                    .heads()
                                    .iter()
                                    .map(|m| self._graph.get_module(*m).unwrap().name())
                                    .collect(),
                                route
                                    .middle()
                                    .iter()
                                    .map(|m| self._graph.get_module(*m).unwrap().name())
                                    .collect(),
                                route
                                    .tails()
                                    .iter()
                                    .map(|m| self._graph.get_module(*m).unwrap().name())
                                    .collect(),
                            )
                        })
                        .collect(),
                )
            })
            .sorted()
            .collect::<Vec<_>>();

        self.convert_package_dependencies_to_python(py, illegal_dependencies)
    }

    #[pyo3(name = "clone")]
    pub fn clone_py(&self) -> GraphWrapper {
        self.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
struct Import {
    importer: String,
    imported: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
struct ImportDetails {
    importer: String,
    imported: String,
    line_number: u32,
    line_contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
struct PackageDependency {
    importer: String,
    imported: String,
    routes: Vec<Route>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, new)]
struct Route {
    heads: Vec<String>,
    middle: Vec<String>,
    tails: Vec<String>,
}

impl From<ModuleToken> for Vec<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        vec![value]
    }
}

impl From<ModuleToken> for FxHashSet<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        FxHashSet::from_iter([value])
    }
}

pub trait ExtendWithDescendants:
    Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>
{
    /// Extend this collection of module tokens with all descendant items.
    fn extend_with_descendants(&mut self, graph: &Graph) {
        for item in self.clone().into_iter() {
            let descendants = graph.get_module_descendants(item).map(|item| item.token());
            self.extend(descendants);
        }
    }

    /// Extend this collection of module tokens with all descendant items.
    fn with_descendants(mut self, graph: &Graph) -> Self {
        self.extend_with_descendants(graph);
        self
    }
}

impl<T: Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>>
    ExtendWithDescendants for T
{
}

pub trait ModuleIterator<'a>: Iterator<Item = &'a Module> + Sized {
    fn tokens(self) -> impl Iterator<Item = ModuleToken> {
        self.map(|m| m.token)
    }

    fn interned_names(self) -> impl Iterator<Item = DefaultSymbol> {
        self.map(|m| m.interned_name)
    }

    fn names(self) -> impl Iterator<Item = String> {
        let interner = MODULE_NAMES.read().unwrap();
        self.map(move |m| interner.resolve(m.interned_name).unwrap().to_owned())
    }

    fn visible(self) -> impl ModuleIterator<'a> {
        self.filter(|m| !m.is_invisible)
    }
}

impl<'a, T: Iterator<Item = &'a Module>> ModuleIterator<'a> for T {}

pub trait ModuleTokenIterator<'a>: Iterator<Item = &'a ModuleToken> + Sized {
    fn into_module_iterator(self, graph: &'a Graph) -> impl ModuleIterator<'a> {
        self.map(|m| graph.get_module(*m).unwrap())
    }
}

impl<'a, T: Iterator<Item = &'a ModuleToken>> ModuleTokenIterator<'a> for T {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, new, Getters, CopyGetters)]
pub struct PyImportDetails {
    #[getset(get_copy = "pub")]
    line_number: u32,

    #[getset(get_copy = "pub")]
    interned_line_contents: DefaultSymbol,
}

impl PyImportDetails {
    pub fn line_contents(&self) -> String {
        let interner = IMPORT_LINE_CONTENTS.read().unwrap();
        interner
            .resolve(self.interned_line_contents)
            .unwrap()
            .to_owned()
    }
}
