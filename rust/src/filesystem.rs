use pyo3::exceptions::PyFileNotFoundError;
use pyo3::prelude::*;
use std::collections::HashMap;
use unindent::unindent;

type FileSystemContents = HashMap<String, String>;

// Implements BasicFileSystem (defined in grimp.application.ports.filesystem.BasicFileSystem).
#[pyclass]
pub struct FakeBasicFileSystem {
    contents: FileSystemContents,
}

#[pymethods]
impl FakeBasicFileSystem {
    #[pyo3(signature = (contents=None, content_map=None))]
    #[new]
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
            contents: parsed_contents,
        })
    }

    #[getter]
    fn sep(&self) -> String {
        "/".to_string()
    }

    #[pyo3(signature = (*components))]
    fn join(&self, components: Vec<String>) -> String {
        let sep = self.sep();
        components
            .into_iter()
            .map(|c| c.trim_end_matches(&sep).to_string())
            .collect::<Vec<String>>()
            .join(&sep)
    }

    fn split(&self, file_name: &str) -> (String, String) {
        let components: Vec<&str> = file_name.split('/').collect();

        if components.is_empty() {
            return ("".to_string(), "".to_string());
        }

        let tail = components.last().unwrap_or(&""); // Last component, or empty if components is empty (shouldn't happen from split)

        let head_components = &components[..components.len() - 1]; // All components except the last

        let head = if head_components.is_empty() {
            // Case for single component paths like "filename.txt" or empty string ""
            "".to_string()
        } else if file_name.starts_with('/')
            && head_components.len() == 1
            && head_components[0].is_empty()
        {
            // Special handling for paths starting with '/', e.g., "/" or "/filename.txt"
            // If components were ["", ""], head_components is [""] -> should be "/"
            // If components were ["", "file.txt"], head_components is [""] -> should be "/"
            "/".to_string()
        } else {
            // Default joining for multiple components
            head_components.join("/")
        };

        (head, tail.to_string())
    }

    /// Checks if a file or directory exists within the file system.
    fn exists(&self, file_name: &str) -> bool {
        self.contents.contains_key(file_name)
    }

    fn read(&self, file_name: &str) -> PyResult<String> {
        match self.contents.get(file_name) {
            Some(file_name) => Ok(file_name.clone()),
            None => Err(PyFileNotFoundError::new_err("")),
        }
    }
}

/// Parses an indented string representing a file system structure
/// into a HashMap where keys are full file paths.
/// See tests.adaptors.filesystem.FakeFileSystem for the API.
pub fn parse_indented_file_system_string(file_system_string: &str) -> HashMap<String, String> {
    let mut file_paths_map: HashMap<String, String> = HashMap::new();
    let mut path_stack: Vec<String> = Vec::new(); // Stores current directory path components
    let mut first_line = true; // Flag to handle the very first path component

    // Normalize newlines and split into lines
    let buffer = file_system_string.replace("\r\n", "\n");
    let lines: Vec<&str> = buffer.split('\n').collect();

    for line_raw in lines.clone() {
        let line = line_raw.trim_end(); // Remove trailing whitespace
        if line.is_empty() {
            continue; // Skip empty lines
        }

        let current_indent = line.chars().take_while(|&c| c.is_whitespace()).count();
        let trimmed_line = line.trim_start();

        // Assuming 4 spaces per indentation level for calculating depth
        // Adjust this if your indentation standard is different (e.g., 2 spaces, tabs)
        let current_depth = current_indent / 4;

        if first_line {
            // The first non-empty line sets the base path.
            // It might be absolute (/a/b/) or relative (a/b/).
            let root_component = trimmed_line.trim_end_matches('/').to_string();
            path_stack.push(root_component);
            first_line = false;
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
            if lines[0].trim().starts_with('/') && !joined.starts_with('/') {
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
        && !path_stack.last().unwrap().ends_with('/')
        && !file_paths_map.contains_key(&path_stack.join("/"))
    {
        file_paths_map.insert(path_stack.join("/"), String::new());
    }

    file_paths_map
}
