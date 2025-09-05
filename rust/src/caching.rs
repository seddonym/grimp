use crate::errors::{GrimpError, GrimpResult};
use crate::filesystem::get_file_system_boxed;
use crate::import_scanning::{DirectImport, imports_by_module_to_py};
use crate::module_finding::Module;
use pyo3::types::PyDict;
use pyo3::{Bound, PyAny, PyResult, Python, pyfunction};
use std::collections::{HashMap, HashSet};

/// Reads the cache file containing all the imports for a given package.
/// Args:
/// - filename: str
/// - file_system: The file system interface to use. (A BasicFileSystem.)
/// Returns Dict[Module, Set[DirectImport]]
#[pyfunction]
pub fn read_cache_data_map_file<'py>(
    py: Python<'py>,
    filename: &str,
    file_system: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let file_system_boxed = get_file_system_boxed(&file_system)?;

    let file_contents = file_system_boxed.read(filename)?;

    let imports_by_module = parse_json_to_map(&file_contents, filename)?;

    Ok(imports_by_module_to_py(py, imports_by_module))
}

pub fn parse_json_to_map(
    json_str: &str,
    filename: &str,
) -> GrimpResult<HashMap<Module, HashSet<DirectImport>>> {
    let raw_map: HashMap<String, Vec<(String, usize, String)>> = serde_json::from_str(json_str)
        .map_err(|_| GrimpError::CorruptCache(filename.to_string()))?;

    let mut parsed_map: HashMap<Module, HashSet<DirectImport>> = HashMap::new();

    for (module_name, imports) in raw_map {
        let module = Module {
            name: module_name.clone(),
        };
        let import_set: HashSet<DirectImport> = imports
            .into_iter()
            .map(|(imported, line_number, line_contents)| DirectImport {
                importer: module_name.clone(),
                imported,
                line_number,
                line_contents,
            })
            .collect();
        parsed_map.insert(module, import_set);
    }

    Ok(parsed_map)
}
