use std::collections::HashMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use bincode::{Decode, Encode};

use crate::errors::{GrimpError, GrimpResult};
use crate::import_parsing::ImportedObject;

/// Cache for storing parsed import information.
#[derive(Debug, Clone)]
pub struct ImportsCache {
    cache_dir: PathBuf,
    // Map of package name to package imports.
    cache: HashMap<String, HashMap<String, CachedImports>>,
}

impl ImportsCache {
    /// Get cached imports for a module if they exist and the mtime matches.
    pub fn get_imports(
        &self,
        package_name: &str,
        module_name: &str,
        mtime_secs: i64,
    ) -> Option<Vec<ImportedObject>> {
        let package_cache = self.cache.get(package_name)?;
        let cached = package_cache.get(module_name)?;
        if cached.mtime_secs() == mtime_secs {
            Some(cached.imported_objects().to_vec())
        } else {
            None
        }
    }

    /// Store parsed imports for a module.
    pub fn set_imports(
        &mut self,
        package_name: String,
        module_name: String,
        mtime_secs: i64,
        imports: Vec<ImportedObject>,
    ) {
        self.cache
            .entry(package_name)
            .or_default()
            .insert(module_name, CachedImports::new(mtime_secs, imports));
    }

    /// Save cache to disk.
    pub fn save(&self) -> GrimpResult<()> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| GrimpError::CacheWriteError {
            path: self.cache_dir.display().to_string(),
            error: e.to_string(),
        })?;

        // Write marker files if they don't exist
        self.write_marker_files_if_missing()?;

        for (package_name, package_cache) in &self.cache {
            let cache_file = self
                .cache_dir
                .join(format!("{}.imports.bincode", package_name));

            let encoded = bincode::encode_to_vec(package_cache, bincode::config::standard())
                .map_err(|e| GrimpError::CacheWriteError {
                    path: cache_file.display().to_string(),
                    error: e.to_string(),
                })?;

            let mut file =
                fs::File::create(&cache_file).map_err(|e| GrimpError::CacheWriteError {
                    path: cache_file.display().to_string(),
                    error: e.to_string(),
                })?;

            file.write_all(&encoded)
                .map_err(|e| GrimpError::CacheWriteError {
                    path: cache_file.display().to_string(),
                    error: e.to_string(),
                })?;
        }

        Ok(())
    }

    /// Write marker files (.gitignore and CACHEDIR.TAG) if they don't already exist.
    fn write_marker_files_if_missing(&self) -> GrimpResult<()> {
        let marker_files = [
            (".gitignore", "# Automatically created by Grimp.\n*"),
            (
                "CACHEDIR.TAG",
                "Signature: 8a477f597d28d172789f06886806bc55\n\
                 # This file is a cache directory tag automatically created by Grimp.\n\
                 # For information about cache directory tags see https://bford.info/cachedir/",
            ),
        ];

        for (filename, contents) in &marker_files {
            let full_path = self.cache_dir.join(filename);
            if !full_path.exists() {
                fs::write(&full_path, contents).map_err(|e| GrimpError::CacheWriteError {
                    path: full_path.display().to_string(),
                    error: e.to_string(),
                })?;
            }
        }

        Ok(())
    }

    /// Load cache from disk.
    pub fn load(cache_dir: &Path, package_names: &[String]) -> Self {
        let mut cache = HashMap::new();

        for package_name in package_names {
            let cache_file = cache_dir.join(format!("{}.imports.bincode", package_name));

            let package_cache = if let Ok(mut file) = fs::File::open(&cache_file) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    if let Ok(decoded) = bincode::decode_from_slice::<
                        HashMap<String, CachedImports>,
                        _,
                    >(&buffer, bincode::config::standard())
                    {
                        decoded.0
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            };

            cache.insert(package_name.clone(), package_cache);
        }

        ImportsCache {
            cache,
            cache_dir: cache_dir.to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
struct CachedImports {
    mtime_secs: i64,
    imported_objects: Vec<ImportedObject>,
}

impl CachedImports {
    fn new(mtime_secs: i64, imported_objects: Vec<ImportedObject>) -> Self {
        Self {
            mtime_secs,
            imported_objects,
        }
    }

    fn mtime_secs(&self) -> i64 {
        self.mtime_secs
    }

    fn imported_objects(&self) -> &[ImportedObject] {
        &self.imported_objects
    }
}
