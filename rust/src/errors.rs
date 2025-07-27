use crate::exceptions::{InvalidModuleExpression, ModuleNotPresent, NoSuchContainer, ParseError};
use pyo3::PyErr;
use pyo3::exceptions::PyValueError;
use ruff_python_parser::ParseError as RuffParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrimpError {
    #[error("Module {0} is not present in the graph.")]
    ModuleNotPresent(String),

    #[error("Container {0} does not exist.")]
    NoSuchContainer(String),

    #[error("Modules have shared descendants.")]
    SharedDescendants,

    #[error("{0} is not a valid module expression.")]
    InvalidModuleExpression(String),

    #[error("Error parsing python code (line {line_number}, text {text}).")]
    ParseError {
        line_number: usize,
        text: String,
        #[source]
        parse_error: RuffParseError,
    },
}

pub type GrimpResult<T> = Result<T, GrimpError>;

impl From<GrimpError> for PyErr {
    fn from(value: GrimpError) -> Self {
        // A default mapping from `GrimpError`s to python exceptions.
        match value {
            GrimpError::ModuleNotPresent(_) => ModuleNotPresent::new_err(value.to_string()),
            GrimpError::NoSuchContainer(_) => NoSuchContainer::new_err(value.to_string()),
            GrimpError::SharedDescendants => PyValueError::new_err(value.to_string()),
            GrimpError::InvalidModuleExpression(_) => {
                InvalidModuleExpression::new_err(value.to_string())
            }
            GrimpError::ParseError {
                line_number, text, ..
            } => PyErr::new::<ParseError, _>((line_number, text)),
        }
    }
}
