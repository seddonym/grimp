use itertools::Itertools;
use pyo3::exceptions::{PyFileNotFoundError, PyTypeError, PyUnicodeDecodeError};
use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use unindent::unindent;

static ENCODING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t\f]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)").unwrap());

pub trait FileSystem: Send + Sync {
    fn sep(&self) -> &str;

    fn join(&self, components: Vec<String>) -> String;

    fn split(&self, file_name: &str) -> (String, String);

    fn exists(&self, file_name: &str) -> bool;

    fn read(&self, file_name: &str) -> PyResult<String>;

    fn write(&mut self, file_name: &str, contents: &str) -> PyResult<()>;
}

#[derive(Clone)]
#[pyclass]
struct RealBasicFileSystem {}

// Implements a BasicFileSystem (defined in grimp.application.ports.filesystem.BasicFileSystem)
// that actually reads files.
#[pyclass(name = "RealBasicFileSystem")]
pub struct PyRealBasicFileSystem {
    inner: RealBasicFileSystem,
}

impl FileSystem for RealBasicFileSystem {
    fn sep(&self) -> &str {
        std::path::MAIN_SEPARATOR_STR
    }

    fn join(&self, components: Vec<String>) -> String {
        let mut path = PathBuf::new();
        for component in components {
            path.push(component);
        }
        path.to_str()
            .expect("Path components should be valid unicode")
            .to_string()
    }

    fn split(&self, file_name: &str) -> (String, String) {
        let path = Path::new(file_name);

        // Get the "tail" part (the file name or last directory)
        let tail = match path.file_name() {
            Some(name) => PathBuf::from(name),
            None => PathBuf::new(), // If there's no file name (e.g., path is a root), return empty
        };

        // Get the "head" part (the parent directory)
        let head = match path.parent() {
            Some(parent_path) => parent_path.to_path_buf(),
            None => PathBuf::new(), // If there's no parent (e.g., just a filename), return empty
        };

        (
            head.to_str()
                .expect("Path components should be valid unicode")
                .to_string(),
            tail.to_str()
                .expect("Path components should be valid unicode")
                .to_string(),
        )
    }

    fn exists(&self, file_name: &str) -> bool {
        Path::new(file_name).is_file()
    }

    fn read(&self, file_name: &str) -> PyResult<String> {
        // Python files are assumed UTF-8 by default (PEP 686), but they can specify an alternative
        // encoding, which we need to take into account here.
        // See https://peps.python.org/pep-0263/

        // This method was authored primarily by an LLM.

        let path = Path::new(file_name);
        let bytes = fs::read(path).map_err(|e| {
            PyFileNotFoundError::new_err(format!("Failed to read file {file_name}: {e}"))
        })?;

        let s = String::from_utf8_lossy(&bytes);

        let mut detected_encoding: Option<String> = None;

        // Coding specification needs to be in the first two lines, or it's ignored.
        for line in s.lines().take(2) {
            if let Some(captures) = ENCODING_RE.captures(line)
                && let Some(encoding_name) = captures.get(1)
            {
                detected_encoding = Some(encoding_name.as_str().to_string());
                break;
            }
        }

        if let Some(enc_name) = detected_encoding {
            let encoding =
                encoding_rs::Encoding::for_label(enc_name.as_bytes()).ok_or_else(|| {
                    PyUnicodeDecodeError::new_err(format!(
                        "Failed to decode file {file_name} (unknown encoding '{enc_name}')"
                    ))
                })?;
            let (decoded_s, _, had_errors) = encoding.decode(&bytes);
            if had_errors {
                Err(PyUnicodeDecodeError::new_err(format!(
                    "Failed to decode file {file_name} with encoding '{enc_name}'"
                )))
            } else {
                Ok(decoded_s.into_owned())
            }
        } else {
            // Default to UTF-8 if no encoding is specified
            String::from_utf8(bytes).map_err(|e| {
                PyUnicodeDecodeError::new_err(format!(
                    "Failed to decode file {file_name} as UTF-8: {e}"
                ))
            })
        }
    }

    fn write(&mut self, file_name: &str, contents: &str) -> PyResult<()> {
        let file_path: PathBuf = file_name.into();
        if let Some(patent_dir) = file_path.parent() {
            fs::create_dir_all(patent_dir)?;
        }
        File::create(file_path)?
            .write_all(contents.as_bytes())
            .map_err(Into::into)
    }
}

#[pymethods]
impl PyRealBasicFileSystem {
    #[new]
    fn new() -> Self {
        PyRealBasicFileSystem {
            inner: RealBasicFileSystem {},
        }
    }

    #[getter]
    fn sep(&self) -> &str {
        self.inner.sep()
    }

    #[pyo3(signature = (*components))]
    fn join(&self, components: Vec<String>) -> String {
        self.inner.join(components)
    }

    fn split(&self, file_name: &str) -> (String, String) {
        self.inner.split(file_name)
    }

    fn exists(&self, file_name: &str) -> bool {
        self.inner.exists(file_name)
    }

    fn read(&self, file_name: &str) -> PyResult<String> {
        self.inner.read(file_name)
    }

    fn write(&mut self, file_name: &str, contents: &str) -> PyResult<()> {
        self.inner.write(file_name, contents)
    }
}

type FileSystemContents = HashMap<String, String>;

#[derive(Clone)]
struct FakeBasicFileSystem {
    contents: Arc<Mutex<FileSystemContents>>,
}

// Implements BasicFileSystem (defined in grimp.application.ports.filesystem.BasicFileSystem).
#[pyclass(name = "FakeBasicFileSystem")]
pub struct PyFakeBasicFileSystem {
    inner: FakeBasicFileSystem,
}

impl FakeBasicFileSystem {
    fn new(contents: Option<&str>, content_map: Option<HashMap<String, String>>) -> PyResult<Self> {
        let mut parsed_contents = match contents {
            Some(contents) => parse_indented_file_system_string(contents),
            None => HashMap::new(),
        };
        if let Some(content_map) = content_map {
            let unindented_map: HashMap<String, String> = content_map
                .into_iter()
                .map(|(key, val)| (key, unindent(&val).trim().to_string()))
                .collect();
            parsed_contents.extend(unindented_map);
        };
        Ok(FakeBasicFileSystem {
            contents: Arc::new(Mutex::new(parsed_contents)),
        })
    }
}

impl FileSystem for FakeBasicFileSystem {
    fn sep(&self) -> &str {
        "/"
    }

    fn join(&self, components: Vec<String>) -> String {
        let sep = self.sep();
        components
            .into_iter()
            .map(|c| c.trim_end_matches(sep).to_string())
            .join(sep)
    }

    fn split(&self, file_name: &str) -> (String, String) {
        let path;
        let head;
        let tail;
        if let Some(file_name_without_trailing_slash) = file_name.strip_suffix("/") {
            head = Path::new(file_name_without_trailing_slash);
            tail = OsStr::new("");
        } else {
            path = Path::new(file_name);
            head = path.parent().unwrap_or(Path::new(""));
            tail = path.file_name().unwrap_or(OsStr::new(""));
        }
        (
            head.to_str()
                .expect("Path components should be valid unicode")
                .to_string(),
            tail.to_str()
                .expect("Path components should be valid unicode")
                .to_string(),
        )
    }

    /// Checks if a file or directory exists within the file system.
    fn exists(&self, file_name: &str) -> bool {
        self.contents.lock().unwrap().contains_key(file_name)
    }

    fn read(&self, file_name: &str) -> PyResult<String> {
        let contents = self.contents.lock().unwrap();
        match contents.get(file_name) {
            Some(file_contents) => Ok(file_contents.clone()),
            None => Err(PyFileNotFoundError::new_err(format!(
                "No such file: {file_name}"
            ))),
        }
    }

    #[allow(unused_variables)]
    fn write(&mut self, file_name: &str, contents: &str) -> PyResult<()> {
        let mut contents_mut = self.contents.lock().unwrap();
        contents_mut.insert(file_name.to_string(), contents.to_string());
        Ok(())
    }
}

#[pymethods]
impl PyFakeBasicFileSystem {
    #[pyo3(signature = (contents=None, content_map=None))]
    #[new]
    fn new(contents: Option<&str>, content_map: Option<HashMap<String, String>>) -> PyResult<Self> {
        Ok(PyFakeBasicFileSystem {
            inner: FakeBasicFileSystem::new(contents, content_map)?,
        })
    }

    #[getter]
    fn sep(&self) -> &str {
        self.inner.sep()
    }

    #[pyo3(signature = (*components))]
    fn join(&self, components: Vec<String>) -> String {
        self.inner.join(components)
    }

    fn split(&self, file_name: &str) -> (String, String) {
        self.inner.split(file_name)
    }

    /// Checks if a file or directory exists within the file system.
    fn exists(&self, file_name: &str) -> bool {
        self.inner.exists(file_name)
    }

    fn read(&self, file_name: &str) -> PyResult<String> {
        self.inner.read(file_name)
    }

    fn write(&mut self, file_name: &str, contents: &str) -> PyResult<()> {
        self.inner.write(file_name, contents)
    }

    // Temporary workaround method for Python tests.
    fn convert_to_basic(&self) -> PyResult<Self> {
        Ok(PyFakeBasicFileSystem {
            inner: self.inner.clone(),
        })
    }
}

/// Parses an indented string representing a file system structure
/// into a HashMap where keys are full file paths.
/// See tests.adaptors.filesystem.FakeFileSystem for the API.
fn parse_indented_file_system_string(file_system_string: &str) -> HashMap<String, String> {
    let mut file_paths_map: HashMap<String, String> = HashMap::new();
    let mut path_stack: Vec<String> = Vec::new(); // Stores current directory path components
    let mut first_line = true; // Flag to handle the very first path component
    let mut first_line_indent: usize = 0;
    // Normalize newlines and split into lines
    let buffer = file_system_string.replace("\r\n", "\n");
    let lines: Vec<&str> = buffer.split('\n').collect();

    let first_line_starts_with_slash = lines[0].trim().starts_with('/');
    for line_raw in lines {
        let line = line_raw.trim_end(); // Remove trailing whitespace
        if line.is_empty() {
            continue; // Skip empty lines
        }
        let current_indent =
            line.chars().take_while(|&c| c.is_whitespace()).count() - first_line_indent;
        let trimmed_line = line.trim_start();

        // Assuming 4 spaces per indentation level for calculating depth
        let current_depth = current_indent / 4;
        if first_line {
            // The first non-empty line sets the base path.
            // It might be absolute (/a/b/) or relative (a/b/).
            let root_component = trimmed_line.trim_end_matches('/').to_string();
            path_stack.push(root_component);
            first_line = false;
            first_line_indent = current_indent;
        } else {
            // Adjust the path_stack based on indentation level
            // Pop elements from the stack until we reach the correct parent directory depth
            while path_stack.len() > current_depth {
                path_stack.pop();
            }
            // If the current line is a file, append it to the path for inserting into map,
            // then pop it off so that subsequent siblings are correctly handled.
            // If it's a directory, append it and it stays on the stack for its children.
            let component_to_add = trimmed_line.trim_end_matches('/').to_string();
            if !component_to_add.is_empty() {
                // Avoid pushing empty strings due to lines like just "/"
                path_stack.push(component_to_add);
            }
        }

        // Construct the full path
        // Join components on the stack. If the first component started with '/',
        // ensure the final path also starts with '/'.
        let full_path = if !path_stack.is_empty() {
            let mut joined = path_stack.join("/");
            // If the original root started with a slash, ensure the final path does too.
            // But be careful not to double-slash if a component is e.g. "/root"
            if first_line_starts_with_slash && !joined.starts_with('/') {
                joined = format!("/{joined}");
            }
            joined
        } else {
            "".to_string()
        };

        // If it's a file (doesn't end with '/'), add it to the map
        // A file is not a directory, so its name should be removed from the stack after processing
        // so that sibling items are at the correct level.
        if !trimmed_line.ends_with('/') {
            file_paths_map.insert(full_path, String::new()); // Value can be empty or actual content
            if !path_stack.is_empty() {
                path_stack.pop(); // Pop the file name off the stack
            }
        }
    }

    // Edge case: If the very first line was a file and it ended up on the stack, it needs to be processed.
    // This handles single-file inputs like "myfile.txt"
    if !path_stack.is_empty()
        && !path_stack
            .last()
            .expect("path_stack should be non-empty")
            .ends_with('/')
        && !file_paths_map.contains_key(&path_stack.join("/"))
    {
        file_paths_map.insert(path_stack.join("/"), String::new());
    }

    file_paths_map
}

#[allow(clippy::borrowed_box)]
pub fn get_file_system_boxed<'py>(
    file_system: &Bound<'py, PyAny>,
) -> PyResult<Box<dyn FileSystem + Send + Sync>> {
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

    Ok(file_system_boxed)
}
