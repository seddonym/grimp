use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::fs;
use std::io;
use std::collections::HashMap;
use std::ops::Add;

#[pyclass]
pub struct RealFileSystem {}


#[pymethods]
impl RealFileSystem {
    #[new]
    fn new() -> Self {
        RealFileSystem {}
    }

    fn read(&self, filename: &str) -> Result<String, io::Error> {
        fs::read_to_string(filename)
    }
}


// Define a type alias for the nested HashMap structure
// This allows the structure to represent arbitrary nested maps,
// similar to Python's dictionary.
type FileSystemContents = HashMap<String, FileNode>;

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
#[serde(untagged)] // Allows deserialization into either map or null
enum FileNode {
    /// A directory, containing more file system contents.
    Directory(FileSystemContents),
    /// A file, represented by `None` in the Python example.
    File(Option<String>),
}

#[pyclass]
pub struct FakeFileSystem {
    contents: FileSystemContents,
}

impl FakeFileSystem {
    /// Helper method to dedent lines, similar to the Python example.
    /// This is a simplified dedent and might need more robust implementation
    /// depending on the exact requirements of the original Python's _dedent method.
    fn _dedent(lines: Vec<&str>) -> Vec<String> {
        if lines.is_empty() {
            return Vec::new();
        }

        // Find the minimum indentation of non-empty lines
        let min_indent = lines
            .iter()
            .filter_map(|line| {
                if line.trim().is_empty() {
                    None
                } else {
                    Some(line.chars().take_while(|&c| c.is_whitespace()).count())
                }
            })
            .min()
            .unwrap_or(0); // If no non-empty lines, assume 0 indentation

        lines
            .iter()
            .map(|line| {
                if line.len() >= min_indent {
                    line[min_indent..].to_string()
                } else {
                    line.to_string() // Should not happen with correct min_indent calculation
                }
            })
            .collect()
    }

    /// Parses raw contents into a nested dictionary-like structure.
    ///
    /// This method expects raw_contents to be a string where indentation defines
    /// the hierarchy, similar to the original Python function.
    ///
    /// Returns a Rust `Result` containing the `FileSystemContents` or an error `String`.
    fn parse_contents(
        raw_contents: &str,
    ) -> PyResult<FileSystemContents> {

        let raw_lines: Vec<&str> = raw_contents
            .split('\n')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let dedented_lines = FakeFileSystem::_dedent(raw_lines);

        let mut yamlified_lines: Vec<String> = Vec::new();
        for line in dedented_lines {
            let trimmed_line = line.trim_end_matches('/').to_string();
            let yamlified_line = trimmed_line.add(":");
            yamlified_lines.push(yamlified_line);
        }

        let yamlified_string = yamlified_lines.join("\n");

        // Use serde_yaml to parse the constructed YAML string
        let parsed_contents: FileSystemContents =
            serde_yaml::from_str(&yamlified_string).map_err(|_e| {
                PyValueError::new_err("Failed to parse YAML from raw_contents")
            })?;

        Ok(parsed_contents)
    }

    /// Helper to join path components.
    fn _join_path(parent: &str, child: &str) -> String {
        if parent.is_empty() {
            child.to_string()
        } else {
            format!("{}/{}", parent, child)
        }
    }
}

#[pyclass]
pub struct FakeFileSystemWalkIterator {
    stack: Vec<(String, FileSystemContents)>,
}

#[pymethods]
impl FakeFileSystem {
    #[new]
    fn new(contents: &str) -> PyResult<Self> {
        let parsed_contents = FakeFileSystem::parse_contents(&contents)?;
        Ok(FakeFileSystem {
            contents: parsed_contents
        })
    }

    #[allow(unused_variables)]
    fn read(&self, filename: &str) -> Result<String, io::Error> {
        panic!("Not yet implemented");
    }

    /// Given a directory, walk the file system recursively.
    ///
    /// For each directory in the tree rooted at directory top (including top itself),
    /// it yields a 3-tuple (dirpath, dirnames, filenames).
    #[pyo3(name = "walk")]
    fn py_walk(&self, directory_name: &str) -> PyResult<FakeFileSystemWalkIterator> {
        let initial_contents = match self.contents.get(directory_name) {
            Some(initial_contents) => initial_contents,
            None => return Ok(FakeFileSystemWalkIterator{stack: vec![]}), 
        };

        let initial_dir_contents = match initial_contents {
            FileNode::Directory(contents) => contents.clone(),
            _ => return Err(PyValueError::new_err("Provided path is not a directory")),
        };

        let a: FileSystemContents = initial_dir_contents.clone();
        Ok(FakeFileSystemWalkIterator {
            stack: vec![(directory_name.to_string(), a)],
        })
    }

    #[getter]
    fn sep(&self) -> String {
        "/".to_string()
    }

    /// Joins path components using the file system separator.
    /// Equivalent to `os.path.join` in Python.
    fn join(&self, components: Vec<String>) -> String {
        let sep = self.sep(); // Get the separator from the getter method
        components.into_iter()
            .map(|c| c.trim_end_matches(&sep).to_string())
            .collect::<Vec<String>>()
            .join(&sep)
    }

    /// Split the path into a pair of (head, tail) where tail is the last
    /// pathname component and head is everything leading up to that.
    ///
    /// This is equivalent to Python's `os.path.split`.
    #[pyo3(name = "split")]
    fn py_split(&self, file_name: &str) -> (String, String) {
        let components: Vec<&str> = file_name.split('/').collect();

        if components.is_empty() {
            return ("".to_string(), "".to_string());
        }

        let tail = components.last().unwrap_or(&""); // Last component, or empty if components is empty (shouldn't happen from split)

        let head_components = &components[..components.len() - 1]; // All components except the last

        let head = if head_components.is_empty() {
            // Case for single component paths like "filename.txt" or empty string ""
            "".to_string()
        } else if file_name.starts_with('/') && head_components.len() == 1 && head_components[0].is_empty() {
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

    /// Return the full path to the directory name of the supplied filename.
    ///
    /// E.g. '/path/to/filename.py' will return '/path/to'.
    #[pyo3(name = "dirname")]
    fn py_dirname(&self, filename: &str) -> String {
        self.py_split(filename).0 // Get the first element (head) from the split result
    }
}


#[pymethods]
impl FakeFileSystemWalkIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(String, Vec<String>, Vec<String>)> {
        while let Some((current_dir_path, current_dir_contents)) = self.stack.pop() {
            let mut directories = Vec::new();
            let mut files = Vec::new();

            for (key, value) in current_dir_contents.iter() {
                match value {
                    FileNode::Directory(_) => directories.push(key.clone()),
                    FileNode::File(_) => files.push(key.clone()),
                }
            }

            // Sort for consistent output, matching typical file system walk behavior
            directories.sort();
            files.sort();

            // Push subdirectories onto the stack in reverse order so they are processed in
            // lexicographical order (LIFO from stack pop)
            for dir_name in directories.iter().rev() {
                if let Some(FileNode::Directory(subdir_contents)) = current_dir_contents.get(dir_name) {
                    let full_subdir_path = FakeFileSystem::_join_path(&current_dir_path, dir_name);
                    self.stack.push((full_subdir_path, subdir_contents.clone()));
                }
            }
            return Some((current_dir_path, directories, files));
        }
        None
    }
}