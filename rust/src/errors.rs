use crate::exceptions;
use pyo3::PyErr;
use pyo3::exceptions::{PyFileNotFoundError, PyIOError, PyValueError};
use ruff_python_parser::ParseError as RuffParseError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Module {0} is not present in the graph.")]
pub struct ModuleNotPresent(pub String);

impl From<ModuleNotPresent> for PyErr {
    fn from(value: ModuleNotPresent) -> Self {
        exceptions::ModuleNotPresent::new_err(value.to_string())
    }
}

#[derive(Debug, Error)]
pub enum GrimpError {
    #[error("Container {0} does not exist.")]
    NoSuchContainer(String),

    #[error("Modules have shared descendants.")]
    SharedDescendants,

    #[error("{0} is not a valid module expression.")]
    InvalidModuleExpression(String),

    #[error("Error parsing python code (line {line_number}, text {text}).")]
    ParseError {
        module_filename: String,
        line_number: usize,
        text: String,
        #[source]
        parse_error: RuffParseError,
    },

    #[error("Could not use corrupt cache file {0}.")]
    CorruptCache(String),

    #[error("Error walking directory: {error}")]
    WalkDirError { error: String },

    #[error("Failed to read file {path}: {error}")]
    FileReadError { path: String, error: String },

    #[error("Failed to get file metadata for {path}: {error}")]
    FileMetadataError { path: String, error: String },

    #[error("Failed to write cache file {path}: {error}")]
    CacheWriteError { path: String, error: String },

    #[error("Package directory does not exist: {0}")]
    PackageDirectoryNotFound(String),
}

pub type GrimpResult<T> = Result<T, GrimpError>;

impl From<GrimpError> for PyErr {
    fn from(value: GrimpError) -> Self {
        // A default mapping from `GrimpError`s to python exceptions.
        match value {
            GrimpError::NoSuchContainer(_) => {
                exceptions::NoSuchContainer::new_err(value.to_string())
            }
            GrimpError::SharedDescendants => PyValueError::new_err(value.to_string()),
            GrimpError::InvalidModuleExpression(_) => {
                exceptions::InvalidModuleExpression::new_err(value.to_string())
            }
            GrimpError::ParseError {
                line_number, text, ..
            } => PyErr::new::<exceptions::ParseError, _>((line_number, text)),
            GrimpError::CorruptCache(_) => exceptions::CorruptCache::new_err(value.to_string()),
            GrimpError::WalkDirError { .. } => PyIOError::new_err(value.to_string()),
            GrimpError::FileReadError { .. } => PyIOError::new_err(value.to_string()),
            GrimpError::FileMetadataError { .. } => PyIOError::new_err(value.to_string()),
            GrimpError::CacheWriteError { .. } => PyIOError::new_err(value.to_string()),
            GrimpError::PackageDirectoryNotFound(_) => {
                PyFileNotFoundError::new_err(value.to_string())
            }
        }
    }
}
