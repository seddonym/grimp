mod caching;
pub mod errors;
pub mod exceptions;
mod filesystem;
pub mod graph;
mod graph_building;
pub mod import_parsing;
mod import_scanning;
pub mod module_expressions;
mod module_finding;

use pyo3::prelude::*;

#[pymodule]
mod _rustgrimp {
    #[pymodule_export]
    use crate::import_scanning::scan_for_imports;

    #[pymodule_export]
    use crate::caching::read_cache_data_map_file;

    #[pymodule_export]
    use crate::caching::write_cache_data_map_file;

    #[pymodule_export]
    use crate::graph::GraphWrapper;

    #[pymodule_export]
    use crate::filesystem::{PyFakeBasicFileSystem, PyRealBasicFileSystem};

    #[pymodule_export]
    use crate::exceptions::{
        CorruptCache, InvalidModuleExpression, ModuleNotPresent, NoSuchContainer, ParseError,
    };

    #[pymodule_export]
    use crate::graph_building::{PyPackageSpec, build_graph_rust};
}
