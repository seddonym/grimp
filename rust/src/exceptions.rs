use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(_rustgrimp, ModuleNotPresent, PyException);
create_exception!(_rustgrimp, NoSuchContainer, PyException);
create_exception!(_rustgrimp, InvalidModuleExpression, PyException);

// We need to use here `pyclass(extends=PyException)` instead of `create_exception!`
// since the exception contains custom data. See:
// * https://github.com/PyO3/pyo3/issues/2597
// * https://github.com/PyO3/pyo3/issues/295
// * https://github.com/PyO3/pyo3/discussions/3259
#[pyclass(extends=PyException)]
pub struct ParseError {
    #[pyo3(get)]
    pub line_number: usize,
    #[pyo3(get)]
    pub text: String,
}

#[pymethods]
impl ParseError {
    #[new]
    fn new(line_number: usize, text: String) -> Self {
        Self { line_number, text }
    }
}
