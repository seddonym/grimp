use pyo3::{prelude::*, types::PyFrozenSet};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, FromPyObject)]
pub struct ModuleFile {
    pub module: Module,
}

/// A Python module.
/// The string is the importable name, e.g. "mypackage.foo".
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, FromPyObject)]
pub struct Module {
    pub name: String,
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Set of modules found under a single package, together with metadata.
pub struct FoundPackage {
    pub name: String,
    pub directory: String,
    // BTreeSet rather than HashSet is necessary to make FoundPackage hashable.
    pub module_files: BTreeSet<ModuleFile>,
}

/// Implements conversion from a Python 'FoundPackage' object to the Rust 'FoundPackage' struct.
/// It extracts 'name', 'directory', and converts the 'module_files' frozenset into a Rust BTreeSet.
impl<'py> FromPyObject<'py> for FoundPackage {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        // Extract the 'name' attribute.
        let name: String = ob.getattr("name")?.extract()?;
        // Extract the 'directory' attribute.
        let directory: String = ob.getattr("directory")?.extract()?;

        // Access the 'module_files' attribute.
        let module_files_py = ob.getattr("module_files")?;
        // Downcast the PyAny object to a PyFrozenSet, as Python 'FrozenSet' maps to 'PyFrozenSet'.
        let py_frozen_set = module_files_py.downcast::<PyFrozenSet>()?;

        let mut module_files = BTreeSet::new();
        // Iterate over the Python frozenset.
        for py_module_file_any in py_frozen_set.iter() {
            // Extract each element (PyAny) into a Rust 'ModuleFile'.
            let module_file: ModuleFile = py_module_file_any.extract()?;
            module_files.insert(module_file);
        }

        Ok(FoundPackage {
            name,
            directory,
            module_files,
        })
    }
}
