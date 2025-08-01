use crate::errors::GrimpError;
use crate::filesystem::{FileSystem, PyFakeBasicFileSystem, PyRealBasicFileSystem};
use crate::import_parsing;
use crate::module_finding::{FoundPackage, Module};
use itertools::Itertools;
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet};
use std::collections::HashSet;

/// Statically analyses some Python modules for import statements within their shared package.
#[pyclass]
pub struct ImportScanner {
    file_system: Box<dyn FileSystem>,
    found_packages: HashSet<FoundPackage>,
    include_external_packages: bool,
    modules: HashSet<Module>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct DirectImport {
    importer: String,
    imported: String,
    line_number: usize,
    line_contents: String,
}

#[pymethods]
impl ImportScanner {
    /// Python args:
    /// 
    /// - file_system:                   The file system interface to use. (A BasicFileSystem.)
    /// - found_packages:                Set of FoundPackages containing all the modules
    ///                                  for analysis.
    /// - include_external_packages:     Whether to include imports of external modules (i.e.
    ///                                  modules not contained in modules_by_package_directory)
    ///                                  in the results.
    #[allow(unused_variables)]
    #[new]
    #[pyo3(signature = (file_system, found_packages, include_external_packages=false))]
    fn new(
        py: Python,
        file_system: Bound<'_, PyAny>,
        found_packages: Bound<'_, PyAny>,
        include_external_packages: bool,
    ) -> PyResult<Self> {
        let file_system_boxed: Box<dyn FileSystem + Send + Sync>;

        if let Ok(py_real) = file_system.extract::<PyRef<PyRealBasicFileSystem>>() {
            file_system_boxed = Box::new(py_real.inner.clone());
        } else if let Ok(py_fake) = file_system.extract::<PyRef<PyFakeBasicFileSystem>>() {
            file_system_boxed = Box::new(py_fake.inner.clone());
        } else {
            return Err(PyTypeError::new_err(
                "file_system must be an instance of RealBasicFileSystem or FakeBasicFileSystem",
            ));
        }
        let found_packages_rust = _py_found_packages_to_rust(found_packages);
        let modules = _get_modules_from_found_packages(&found_packages_rust);

        Ok(ImportScanner {
            file_system: file_system_boxed,
            found_packages: found_packages_rust,
            modules,
            include_external_packages,
        })
    }

    /// Statically analyses the given module and returns a set of Modules that
    /// it imports.
    #[pyo3(signature = (module, exclude_type_checking_imports=false))]
    fn scan_for_imports<'a>(
        &self,
        py: Python<'a>,
        module: Bound<'_, PyAny>,
        exclude_type_checking_imports: bool,
    ) -> PyResult<Bound<'a, PySet>> {
        let module_rust = module.extract().unwrap();
        let found_package_for_module = self._lookup_found_package_for_module(&module_rust);
        let module_filename =
            self._determine_module_filename(&module_rust, found_package_for_module)?;
        let module_contents = self.file_system.read(&module_filename).unwrap();

        let parse_result = import_parsing::parse_imports_from_code(&module_contents);
        match parse_result {
            Err(GrimpError::ParseError {
                line_number, text, ..
            }) => {
                // TODO: define SourceSyntaxError using pyo3.
                let exceptions_pymodule = PyModule::import(py, "grimp.exceptions").unwrap();
                let py_exception_class = exceptions_pymodule.getattr("SourceSyntaxError").unwrap();
                let exception = py_exception_class
                    .call1((module_filename, line_number, text))
                    .unwrap();
                return Err(PyErr::from_value(exception));
            }
            Err(e) => {
                return Err(e.into());
            }
            _ => (),
        }
        let imported_objects = parse_result.unwrap();
        let is_package = self._module_is_package(&module_filename);

        let mut imports: HashSet<DirectImport> = HashSet::new();
        for imported_object in imported_objects {
            // Don't include type checking imports, if specified.
            if exclude_type_checking_imports && imported_object.typechecking_only {
                continue;
            }

            // Resolve relative imports.
            let imported_object_name = self._get_absolute_imported_object_name(
                &module_rust,
                is_package,
                &imported_object.name,
            );

            // Resolve imported module.
            match self._get_internal_module(&imported_object_name) {
                Some(imported_module) => {
                    // It's an internal import.
                    imports.insert(DirectImport {
                        importer: module_rust.name.to_string(),
                        imported: imported_module.name.to_string(),
                        line_number: imported_object.line_number,
                        line_contents: imported_object.line_contents,
                    });
                }
                None => {
                    // It's an external import.
                    if self.include_external_packages {
                        if let Some(imported_module) =
                            self._distill_external_module(&imported_object_name)
                        {
                            imports.insert(DirectImport {
                                importer: module_rust.name.to_string(),
                                imported: imported_module,
                                line_number: imported_object.line_number,
                                line_contents: imported_object.line_contents,
                            });
                        }
                    }
                }
            }
        }

        Ok(self._to_py_direct_imports(py, &imports))
    }
}

impl ImportScanner {
    fn _lookup_found_package_for_module(&self, module: &Module) -> &FoundPackage {
        // TODO: it's probably inefficient to do this every time we look up a module.
        for found_package in &self.found_packages {
            for module_file in &found_package.module_files {
                if module_file.module == *module {
                    return found_package;
                }
            }
        }
        panic!("Could not lookup found package for module {module}");
    }

    fn _determine_module_filename(
        &self,
        module: &Module,
        found_package: &FoundPackage,
    ) -> PyResult<String> {
        // TODO: could we do all this with &str instead of String?
        let top_level_components: Vec<String> =
            found_package.name.split(".").map(str::to_string).collect();
        let module_components: Vec<String> = module.name.split(".").map(str::to_string).collect();
        let leaf_components = module_components[top_level_components.len()..].to_vec();

        let mut filename_root_components: Vec<String> = vec![found_package.directory.clone()];
        filename_root_components.extend(leaf_components);
        let filename_root = self.file_system.join(filename_root_components);
        let normal_module_path = format!("{filename_root}.py");
        let init_module_path = self
            .file_system
            .join(vec![filename_root, "__init__.py".to_string()]);
        let candidate_filenames = [normal_module_path, init_module_path];
        for candidate_filename in candidate_filenames {
            if self.file_system.exists(&candidate_filename) {
                return Ok(candidate_filename);
            }
        }
        Err(PyFileNotFoundError::new_err(format!(
            "Could not find module {module}."
        )))
    }

    fn _module_is_package(&self, module_filename: &str) -> bool {
        self.file_system.split(module_filename).1 == "__init__.py"
    }

    #[allow(unused_variables)]
    fn _get_absolute_imported_object_name(
        &self,
        module: &Module,
        is_package: bool,
        imported_object_name: &str,
    ) -> String {
        let leading_dots_count = count_leading_dots(imported_object_name);
        if leading_dots_count == 0 {
            return imported_object_name.to_string();
        }
        let imported_object_name_base: String;
        if is_package {
            if leading_dots_count == 1 {
                imported_object_name_base = module.name.clone();
            } else {
                let parts: Vec<&str> = module.name.split('.').collect();
                imported_object_name_base =
                    parts[0..parts.len() - leading_dots_count + 1].join(".");
            }
        } else {
            let parts: Vec<&str> = module.name.split('.').collect();
            imported_object_name_base = parts[0..parts.len() - leading_dots_count].join(".");
        }

        format!(
            "{}.{}",
            imported_object_name_base,
            &imported_object_name[leading_dots_count..]
        )
    }

    #[allow(unused_variables)]
    fn _get_internal_module(&self, imported_object_name: &str) -> Option<Module> {
        let candidate_module = Module {
            name: imported_object_name.to_string(),
        };
        if self.modules.contains(&candidate_module) {
            return Some(candidate_module);
        }
        if let Some((parent_name, _)) = imported_object_name.rsplit_once('.') {
            let parent = Module {
                name: parent_name.to_string(),
            };
            if self.modules.contains(&parent) {
                return Some(parent);
            }
        }
        None
    }

    /// Given a module that we already know is external, turn it into a module to add to the graph.
    //
    //  The 'distillation' process involves removing any unwanted subpackages. For example,
    //  django.models.db should be turned into simply django.
    //  The process is more complex for potential namespace packages, as it's not possible to
    //  determine the portion package simply from name. Rather than adding the overhead of a
    //  filesystem read, we just get the shallowest component that does not clash with an internal
    //  module namespace. Take, for example, foo.blue.alpha.one. If one of the found
    //  packages is foo.blue.beta, the module will be distilled to foo.blue.alpha.
    //  Alternatively, if the found package is foo.green, the distilled module will
    //  be foo.blue.
    #[allow(unused_variables)]
    fn _distill_external_module(&self, module_name: &str) -> Option<String> {
        let module_root = module_name.split(".").next().unwrap();
        // If it's a module that is a parent of one of the internal packages, return None
        // as it doesn't make sense and is probably an import of a namespace package.
        for package in &self.found_packages {
            if module_is_descendant(&package.name, module_name) {
                return None;
            }
        }

        // If it shares a namespace with an internal module, get the shallowest component that does
        // not clash with an internal module namespace.
        let mut candidate_portions: HashSet<Module> = HashSet::new();
        let mut sorted_found_packages: Vec<&FoundPackage> = self.found_packages.iter().collect();
        sorted_found_packages.sort_by_key(|package| &package.name);
        sorted_found_packages.reverse();

        for found_package in sorted_found_packages {
            let root_module = &found_package.name;
            if module_is_descendant(root_module, module_root) {
                let mut internal_path_components: Vec<&str> = root_module.split(".").collect();
                let mut external_path_components: Vec<&str> = module_name.split(".").collect();
                let mut external_namespace_components: Vec<&str> = vec![];
                while external_path_components[0] == internal_path_components[0] {
                    external_namespace_components.push(external_path_components.remove(0));
                    internal_path_components.remove(0);
                }
                external_namespace_components.push(external_path_components[0]);

                candidate_portions.insert(Module {
                    name: external_namespace_components.join("."),
                });
            }
        }

        if !candidate_portions.is_empty() {
            // If multiple found packages share a namespace with this module, use the deepest one
            // as we know that that will be a namespace too.
            let deepest_candidate_portion = candidate_portions
                .iter()
                .sorted_by_key(|portion| portion.name.split(".").collect::<Vec<_>>().len())
                .next_back()
                .unwrap();
            Some(deepest_candidate_portion.name.clone())
        } else {
            Some(module_name.split('.').next().unwrap().to_string())
        }
    }

    fn _to_py_direct_imports<'a>(
        &self,
        py: Python<'a>,
        rust_imports: &HashSet<DirectImport>,
    ) -> Bound<'a, PySet> {
        // TODO: do this in the constructor.
        let valueobjects_pymodule = PyModule::import(py, "grimp.domain.valueobjects").unwrap();
        let py_module_class = valueobjects_pymodule.getattr("Module").unwrap();
        let py_direct_import_class = valueobjects_pymodule.getattr("DirectImport").unwrap();

        let pyset = PySet::empty(py).unwrap();

        for rust_import in rust_imports {
            let importer = py_module_class.call1((&rust_import.importer,)).unwrap();
            let imported = py_module_class.call1((&rust_import.imported,)).unwrap();
            let kwargs = PyDict::new(py);
            kwargs.set_item("importer", &importer).unwrap();
            kwargs.set_item("imported", &imported).unwrap();
            kwargs
                .set_item("line_number", rust_import.line_number)
                .unwrap();
            kwargs
                .set_item("line_contents", &rust_import.line_contents)
                .unwrap();
            let py_direct_import = py_direct_import_class.call((), Some(&kwargs)).unwrap();
            pyset.add(&py_direct_import).unwrap();
        }

        pyset
    }
}
fn _py_found_packages_to_rust(py_found_packages: Bound<'_, PyAny>) -> HashSet<FoundPackage> {
    let py_set = py_found_packages
        .downcast::<PySet>()
        .expect("Expected py_found_packages to be a Python set.");

    let mut rust_found_packages = HashSet::new();
    // Iterate over the elements in the Python set.
    for py_found_package_any in py_set.iter() {
        // Extract each Python 'FoundPackage' object into a Rust 'FoundPackage' struct.
        // Panics if extraction fails, as specified.
        let rust_found_package: FoundPackage = py_found_package_any
            .extract()
            .expect("Failed to extract Python FoundPackage to Rust FoundPackage.");
        rust_found_packages.insert(rust_found_package);
    }
    rust_found_packages
}

fn _get_modules_from_found_packages(found_packages: &HashSet<FoundPackage>) -> HashSet<Module> {
    let mut modules = HashSet::new();
    for package in found_packages {
        for module_file in &package.module_files {
            modules.insert(module_file.module.clone());
        }
    }
    modules
}

fn count_leading_dots(s: &str) -> usize {
    s.chars().take_while(|&c| c == '.').count()
}

fn module_is_descendant(module_name: &str, potential_ancestor: &str) -> bool {
    module_name.starts_with(&format!("{potential_ancestor}."))
}
