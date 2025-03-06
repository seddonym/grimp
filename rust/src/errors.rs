use crate::exceptions::{InvalidModuleExpression, ModuleNotPresent, NoSuchContainer};
use pyo3::exceptions::PyValueError;
use pyo3::PyErr;
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
        }
    }
}
