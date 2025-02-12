use pyo3::create_exception;

create_exception!(_rustgrimp, ModuleNotPresent, pyo3::exceptions::PyException);
create_exception!(_rustgrimp, NoSuchContainer, pyo3::exceptions::PyException);
create_exception!(
    _rustgrimp,
    InvalidModuleExpression,
    pyo3::exceptions::PyException
);
