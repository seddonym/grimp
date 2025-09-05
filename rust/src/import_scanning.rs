use crate::errors::{GrimpError, GrimpResult};
use crate::filesystem::{FileSystem, get_file_system_boxed};
use crate::module_finding::{FoundPackage, Module};
use crate::{import_parsing, module_finding};
use itertools::Itertools;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet};
/// Statically analyses some Python modules for import statements within their shared package.
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::{self, ErrorKind};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct DirectImport {
    pub importer: String,
    pub imported: String,
    pub line_number: usize,
    pub line_contents: String,
}

pub fn py_found_packages_to_rust(py_found_packages: &Bound<'_, PyAny>) -> HashSet<FoundPackage> {
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

pub fn get_modules_from_found_packages(found_packages: &HashSet<FoundPackage>) -> HashSet<Module> {
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

/// Statically analyses the given module and returns a set of Modules that
/// it imports.
#[allow(clippy::borrowed_box)]
pub fn scan_for_imports_no_py(
    file_system: &Box<dyn FileSystem + Send + Sync>,
    found_packages: &HashSet<FoundPackage>,
    include_external_packages: bool,
    modules: &HashSet<Module>,
    exclude_type_checking_imports: bool,
) -> GrimpResult<HashMap<Module, HashSet<DirectImport>>> {
    let module_packages = get_modules_from_found_packages(found_packages);

    // Assemble a lookup table so we only need to do this once.
    let mut found_packages_by_module = HashMap::new();
    for found_package in found_packages {
        for module_file in &found_package.module_files {
            found_packages_by_module.insert(&module_file.module, found_package);
        }
    }
    let results: GrimpResult<Vec<(Module, HashSet<DirectImport>)>> = modules
        .par_iter()
        .map(|module| {
            let imports = scan_for_imports_no_py_single_module(
                module,
                file_system,
                &found_packages_by_module,
                found_packages,
                &module_packages,
                include_external_packages,
                exclude_type_checking_imports,
            )?;
            Ok((module.clone(), imports))
        })
        .collect();

    results.map(|vec| vec.into_iter().collect())
}

#[allow(clippy::borrowed_box)]
fn scan_for_imports_no_py_single_module(
    module: &Module,
    file_system: &Box<dyn FileSystem + Send + Sync>,
    found_packages_by_module: &HashMap<&Module, &FoundPackage>,
    found_packages: &HashSet<FoundPackage>,
    all_modules: &HashSet<Module>,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> GrimpResult<HashSet<DirectImport>> {
    let mut imports: HashSet<DirectImport> = HashSet::new();
    let found_package_for_module = found_packages_by_module[module];
    let module_filename =
        _determine_module_filename(module, found_package_for_module, file_system).unwrap();
    let module_contents = file_system.read(&module_filename).unwrap();
    let imported_objects =
        import_parsing::parse_imports_from_code(&module_contents, &module_filename)?;

    let is_package = _module_is_package(&module_filename, file_system);

    for imported_object in imported_objects {
        // Don't include type checking imports, if specified.
        if exclude_type_checking_imports && imported_object.typechecking_only {
            continue;
        }

        // Resolve relative imports.
        let imported_object_name =
            _get_absolute_imported_object_name(module, is_package, &imported_object.name);

        // Resolve imported module.
        match _get_internal_module(&imported_object_name, all_modules) {
            Some(imported_module) => {
                // It's an internal import.
                imports.insert(DirectImport {
                    importer: module.name.to_string(),
                    imported: imported_module.name.to_string(),
                    line_number: imported_object.line_number,
                    line_contents: imported_object.line_contents,
                });
            }
            None => {
                // It's an external import.
                if include_external_packages
                    && let Some(imported_module) =
                        _distill_external_module(&imported_object_name, found_packages)
                {
                    imports.insert(DirectImport {
                        importer: module.name.to_string(),
                        imported: imported_module,
                        line_number: imported_object.line_number,
                        line_contents: imported_object.line_contents,
                    });
                }
            }
        }
    }

    Ok(imports)
}

pub fn to_py_direct_imports<'a>(
    py: Python<'a>,
    rust_imports: &HashSet<DirectImport>,
) -> Bound<'a, PySet> {
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

#[allow(clippy::borrowed_box)]
fn _determine_module_filename(
    module: &Module,
    found_package: &FoundPackage,
    file_system: &Box<dyn FileSystem + Send + Sync>,
) -> io::Result<String> {
    // TODO: could we do all this with &str instead of String?
    let top_level_components: Vec<String> =
        found_package.name.split(".").map(str::to_string).collect();
    let module_components: Vec<String> = module.name.split(".").map(str::to_string).collect();
    let leaf_components = module_components[top_level_components.len()..].to_vec();

    let mut filename_root_components: Vec<String> = vec![found_package.directory.clone()];
    filename_root_components.extend(leaf_components);
    let filename_root = file_system.join(filename_root_components);
    let normal_module_path = format!("{filename_root}.py");
    let init_module_path = file_system.join(vec![filename_root, "__init__.py".to_string()]);
    let candidate_filenames = [normal_module_path, init_module_path];
    for candidate_filename in candidate_filenames {
        if file_system.exists(&candidate_filename) {
            return Ok(candidate_filename);
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Could not find module {module}.",
    ))
}

#[allow(clippy::borrowed_box)]
fn _module_is_package(
    module_filename: &str,
    file_system: &Box<dyn FileSystem + Send + Sync>,
) -> bool {
    file_system.split(module_filename).1 == "__init__.py"
}

fn _get_absolute_imported_object_name(
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
            imported_object_name_base = parts[0..parts.len() - leading_dots_count + 1].join(".");
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

fn _get_internal_module(
    imported_object_name: &str,
    all_modules: &HashSet<Module>,
) -> Option<Module> {
    let candidate_module = Module {
        name: imported_object_name.to_string(),
    };
    if all_modules.contains(&candidate_module) {
        return Some(candidate_module);
    }
    if let Some((parent_name, _)) = imported_object_name.rsplit_once('.') {
        let parent = Module {
            name: parent_name.to_string(),
        };
        if all_modules.contains(&parent) {
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
fn _distill_external_module(
    module_name: &str,
    found_packages: &HashSet<FoundPackage>,
) -> Option<String> {
    let module_root = module_name.split(".").next().unwrap();
    // If it's a module that is a parent of one of the internal packages, return None
    // as it doesn't make sense and is probably an import of a namespace package.
    for package in found_packages {
        if module_is_descendant(&package.name, module_name) {
            return None;
        }
    }

    // If it shares a namespace with an internal module, get the shallowest component that does
    // not clash with an internal module namespace.
    let mut candidate_portions: HashSet<Module> = HashSet::new();
    let mut sorted_found_packages: Vec<&FoundPackage> = found_packages.iter().collect();
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

/// Convert the rust data structure into a Python dict[Module, set[DirectImport]].
pub fn imports_by_module_to_py(
    py: Python,
    imports_by_module: HashMap<module_finding::Module, HashSet<DirectImport>>,
) -> Bound<PyDict> {
    let valueobjects_pymodule = PyModule::import(py, "grimp.domain.valueobjects").unwrap();
    let py_module_class = valueobjects_pymodule.getattr("Module").unwrap();

    let imports_by_module_py = PyDict::new(py);
    for (module, imports) in imports_by_module.iter() {
        let py_module_instance = py_module_class.call1((module.name.clone(),)).unwrap();
        let py_imports = to_py_direct_imports(py, imports);
        imports_by_module_py
            .set_item(py_module_instance, py_imports)
            .unwrap();
    }
    imports_by_module_py
}

/// Statically analyses the given module and returns a set of Modules that
/// it imports.
/// Python args:
///
/// - module_files                   The modules to scan.
/// - found_packages:                Set of FoundPackages containing all the modules
///                                  for analysis.
/// - include_external_packages:     Whether to include imports of external modules (i.e.
///                                  modules not contained in modules_by_package_directory)
///                                  in the results.
/// - exclude_type_checking_imports: If True, don't include imports behind TYPE_CHECKING guards.
/// - file_system:                   The file system interface to use. (A BasicFileSystem.)
///
/// Returns dict[Module, set[DirectImport]].
#[pyfunction]
pub fn scan_for_imports<'py>(
    py: Python<'py>,
    module_files: Vec<Bound<'py, PyAny>>,
    found_packages: Bound<'py, PyAny>,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    file_system: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let file_system_boxed = get_file_system_boxed(&file_system)?;
    let found_packages_rust = py_found_packages_to_rust(&found_packages);
    let modules_rust: HashSet<module_finding::Module> = module_files
        .iter()
        .map(|module_file| {
            module_file
                .getattr("module")
                .unwrap()
                .extract::<module_finding::Module>()
                .unwrap()
        })
        .collect();

    let imports_by_module_result = py.detach(|| {
        scan_for_imports_no_py(
            &file_system_boxed,
            &found_packages_rust,
            include_external_packages,
            &modules_rust,
            exclude_type_checking_imports,
        )
    });

    match imports_by_module_result {
        Err(GrimpError::ParseError {
            module_filename,
            line_number,
            text,
            ..
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
    let imports_by_module = imports_by_module_result.unwrap();

    let imports_by_module_py = imports_by_module_to_py(py, imports_by_module);

    Ok(imports_by_module_py)
}
