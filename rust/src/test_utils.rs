use filetime::{FileTime, set_file_mtime};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Default mtime for files (in seconds since Unix epoch)
pub const DEFAULT_MTIME: i64 = 10000;

/// A builder for creating temporary directory structures for testing.
///
/// # Example
///
/// ```
/// use _rustgrimp::test_utils::TempFileSystemBuilder;
///
/// let temp_fs = TempFileSystemBuilder::new(r#"
///     mypackage/
///         __init__.py
///         foo/
///             __init__.py
///             one.py
/// "#)
///     .with_file_content_map([
///         ("mypackage/foo/one.py", "from . import two"),
///     ])
///     .with_file_mtime_map([
///         ("mypackage/foo/one.py", 12340000),
///     ])
///     .build()
///     .unwrap();
///
/// let package_dir = temp_fs.join("mypackage");
/// ```
pub struct TempFileSystemBuilder {
    contents: String,
    content_map: HashMap<String, String>,
    mtime_overrides: HashMap<String, i64>,
}

impl TempFileSystemBuilder {
    /// Create a new builder with the directory structure.
    ///
    /// The string should be formatted with directories ending in `/` and files without.
    /// Indentation determines the hierarchy.
    ///
    /// # Example
    ///
    /// ```
    /// let builder = TempFileSystemBuilder::new(r#"
    ///     mypackage/
    ///         __init__.py
    ///         foo/
    ///             __init__.py
    ///             one.py
    /// "#);
    /// ```
    pub fn new(contents: &str) -> Self {
        Self {
            contents: contents.to_string(),
            content_map: HashMap::new(),
            mtime_overrides: HashMap::new(),
        }
    }

    /// Set the content for a specific file (relative path).
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path to the file from the temp directory root
    /// * `content` - The text content of the file
    ///
    /// # Example
    ///
    /// ```
    /// let builder = TempFileSystemBuilder::new("...")
    ///     .with_file_content("mypackage/foo/one.py", "from . import two");
    /// ```
    pub fn with_file_content(mut self, path: &str, content: &str) -> Self {
        self.content_map
            .insert(path.to_string(), content.to_string());
        self
    }

    /// Set the content for multiple files at once.
    ///
    /// # Arguments
    ///
    /// * `content_map` - An iterator of (path, content) pairs
    ///
    /// # Example
    ///
    /// ```
    /// let builder = TempFileSystemBuilder::new("...")
    ///     .with_file_content_map([
    ///         ("mypackage/foo/one.py", "from . import two"),
    ///         ("mypackage/foo/two.py", "x = 1"),
    ///     ]);
    /// ```
    pub fn with_file_content_map(
        mut self,
        content_map: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.content_map
            .extend(content_map.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Set a custom modification time for a specific file (relative path).
    ///
    /// # Arguments
    ///
    /// * `path` - Relative path to the file from the temp directory root
    /// * `mtime` - Modification time in seconds since Unix epoch
    ///
    /// # Example
    ///
    /// ```
    /// let builder = TempFileSystemBuilder::new("...")
    ///     .with_mtime("mypackage/foo/one.py", 12340000);
    /// ```
    pub fn with_file_mtime(mut self, path: &str, mtime: i64) -> Self {
        self.mtime_overrides.insert(path.to_string(), mtime);
        self
    }

    /// Set custom modification times for multiple files at once.
    ///
    /// # Arguments
    ///
    /// * `mtime_map` - An iterator of (path, mtime) pairs
    ///
    /// # Example
    ///
    /// ```
    /// let builder = TempFileSystemBuilder::new("...")
    ///     .with_file_mtime_map([
    ///         ("mypackage/foo/one.py", 12340000),
    ///         ("mypackage/foo/two.py", 12350000),
    ///     ]);
    /// ```
    pub fn with_file_mtime_map(
        mut self,
        mtime_map: impl IntoIterator<Item = (impl Into<String>, i64)>,
    ) -> Self {
        self.mtime_overrides
            .extend(mtime_map.into_iter().map(|(k, v)| (k.into(), v)));
        self
    }

    /// Build the temporary file system
    pub fn build(self) -> std::io::Result<TempFileSystem> {
        let temp_dir = TempDir::new()?;

        // Create the directory structure
        Self::create_structure(temp_dir.path(), &self.contents)?;

        // Write file contents from content_map
        for (relative_path, content) in &self.content_map {
            let full_path = temp_dir.path().join(relative_path);

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = fs::File::create(&full_path)?;
            file.write_all(content.as_bytes())?;

            // Set default mtime for files created via content_map
            let default_filetime = FileTime::from_unix_time(DEFAULT_MTIME, 0);
            set_file_mtime(&full_path, default_filetime)?;
        }

        // Apply mtime overrides (must come after content_map writes)
        for (relative_path, mtime) in &self.mtime_overrides {
            let full_path = temp_dir.path().join(relative_path);
            if full_path.exists() {
                let filetime = FileTime::from_unix_time(*mtime, 0);
                set_file_mtime(&full_path, filetime)?;
            }
        }

        Ok(TempFileSystem { temp_dir })
    }

    fn create_structure(base_path: &Path, contents: &str) -> std::io::Result<()> {
        let lines: Vec<&str> = contents
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            return Ok(());
        }

        // Calculate minimum indentation to dedent
        let min_indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // Parse the structure
        let mut stack: Vec<(usize, PathBuf)> = vec![(0, base_path.to_path_buf())];

        for line in lines {
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                continue;
            }

            let indent = line.len() - trimmed.len() - min_indent;
            let indent_level = indent / 4; // Assume 4 spaces per level

            // Pop stack until we find the parent
            while stack.len() > indent_level + 1 {
                stack.pop();
            }

            let parent_path = &stack.last().unwrap().1;
            let name = trimmed.trim();

            if name.ends_with('/') {
                // Directory
                let dir_name = name.trim_end_matches('/');
                let dir_path = parent_path.join(dir_name);
                fs::create_dir_all(&dir_path)?;
                stack.push((indent_level, dir_path));
            } else {
                // File
                let file_path = parent_path.join(name);
                let mut file = fs::File::create(&file_path)?;
                // Create empty file
                file.write_all(b"")?;

                // Set default mtime
                let default_filetime = FileTime::from_unix_time(DEFAULT_MTIME, 0);
                set_file_mtime(&file_path, default_filetime)?;
            }
        }

        Ok(())
    }
}

/// A temporary directory structure for testing.
pub struct TempFileSystem {
    temp_dir: TempDir,
}

impl TempFileSystem {
    /// Get the root path of the temporary directory
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Join a path component to the root path
    ///
    /// # Example
    ///
    /// ```
    /// let temp_fs = TempFileSystemBuilder::new("...").build().unwrap();
    /// let package_dir = temp_fs.join("mypackage");
    /// ```
    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.temp_dir.path().join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_structure() {
        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
            "#,
        )
        .build()
        .unwrap();

        let base = temp_fs.path();
        assert!(base.join("mypackage").is_dir());
        assert!(base.join("mypackage/__init__.py").is_file());
        assert!(base.join("mypackage/foo").is_dir());
        assert!(base.join("mypackage/foo/__init__.py").is_file());
        assert!(base.join("mypackage/foo/one.py").is_file());
    }

    #[test]
    fn test_create_with_file_content() {
        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
            "#,
        )
        .with_file_content("mypackage/foo/one.py", "from . import two")
        .build()
        .unwrap();

        let one_py = temp_fs.path().join("mypackage/foo/one.py");
        let content = fs::read_to_string(&one_py).unwrap();
        assert_eq!(content, "from . import two");
    }

    #[test]
    fn test_create_with_content_map() {
        let mut content_map = HashMap::new();
        content_map.insert("mypackage/foo/one.py".to_string(), "import sys".to_string());
        content_map.insert("mypackage/foo/two.py".to_string(), "x = 1".to_string());

        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
                    two.py
            "#,
        )
        .with_file_content_map(content_map)
        .build()
        .unwrap();

        let one_py = temp_fs.path().join("mypackage/foo/one.py");
        let content = fs::read_to_string(&one_py).unwrap();
        assert_eq!(content, "import sys");

        let two_py = temp_fs.path().join("mypackage/foo/two.py");
        let content = fs::read_to_string(&two_py).unwrap();
        assert_eq!(content, "x = 1");
    }

    #[test]
    fn test_create_with_custom_mtimes() {
        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
            "#,
        )
        .with_file_mtime("mypackage/foo/one.py", 12340000)
        .build()
        .unwrap();

        let one_py = temp_fs.path().join("mypackage/foo/one.py");
        let metadata = fs::metadata(&one_py).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime.unix_seconds(), 12340000);

        let init_py = temp_fs.path().join("mypackage/__init__.py");
        let metadata = fs::metadata(&init_py).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime.unix_seconds(), DEFAULT_MTIME);
    }

    #[test]
    fn test_nested_directories() {
        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    two/
                        __init__.py
                        green.py
                        blue.py
            "#,
        )
        .build()
        .unwrap();

        let base = temp_fs.path();
        assert!(base.join("mypackage/foo/two").is_dir());
        assert!(base.join("mypackage/foo/two/green.py").is_file());
        assert!(base.join("mypackage/foo/two/blue.py").is_file());
    }

    #[test]
    fn test_builder_chaining() {
        let temp_fs = TempFileSystemBuilder::new(
            r#"
            mypackage/
                __init__.py
                foo/
                    __init__.py
                    one.py
                    two.py
            "#,
        )
        .with_file_mtime("mypackage/foo/one.py", 11111111)
        .with_file_mtime("mypackage/foo/two.py", 22222222)
        .with_file_content("mypackage/foo/one.py", "# one")
        .with_file_content("mypackage/foo/two.py", "# two")
        .build()
        .unwrap();

        let one_py = temp_fs.path().join("mypackage/foo/one.py");
        let metadata = fs::metadata(&one_py).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime.unix_seconds(), 11111111);
        let content = fs::read_to_string(&one_py).unwrap();
        assert_eq!(content, "# one");

        let two_py = temp_fs.path().join("mypackage/foo/two.py");
        let metadata = fs::metadata(&two_py).unwrap();
        let mtime = FileTime::from_last_modification_time(&metadata);
        assert_eq!(mtime.unix_seconds(), 22222222);
        let content = fs::read_to_string(&two_py).unwrap();
        assert_eq!(content, "# two");
    }
}
