use std::collections::HashMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use bincode::{Decode, Encode};

use crate::errors::{GrimpError, GrimpResult};
use crate::import_parsing::ImportedObject;

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

/// Cache for storing parsed import information indexed by module name.
#[derive(Debug, Clone)]
pub struct ImportsCache {
    cache_dir: PathBuf,
    package_name: String,
    cache: HashMap<String, CachedImports>,
}

impl ImportsCache {
    /// Get cached imports for a module if they exist and the mtime matches.
    pub fn get_imports(&self, module_name: &str, mtime_secs: i64) -> Option<Vec<ImportedObject>> {
        let cached = self.cache.get(module_name)?;
        if cached.mtime_secs() == mtime_secs {
            Some(cached.imported_objects().to_vec())
        } else {
            None
        }
    }

    /// Store parsed imports for a module.
    pub fn set_imports(
        &mut self,
        module_name: String,
        mtime_secs: i64,
        imports: Vec<ImportedObject>,
    ) {
        self.cache
            .insert(module_name, CachedImports::new(mtime_secs, imports));
    }

    /// Save the cache to disk.
    pub fn save(&self) -> GrimpResult<()> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| GrimpError::CacheWriteError {
            path: self.cache_dir.display().to_string(),
            error: e.to_string(),
        })?;

        let cache_file = self
            .cache_dir
            .join(format!("{}.imports.bincode", self.package_name));

        let encoded =
            bincode::encode_to_vec(&self.cache, bincode::config::standard()).map_err(|e| {
                GrimpError::CacheWriteError {
                    path: cache_file.display().to_string(),
                    error: e.to_string(),
                }
            })?;

        let mut file = fs::File::create(&cache_file).map_err(|e| GrimpError::CacheWriteError {
            path: cache_file.display().to_string(),
            error: e.to_string(),
        })?;

        file.write_all(&encoded)
            .map_err(|e| GrimpError::CacheWriteError {
                path: cache_file.display().to_string(),
                error: e.to_string(),
            })?;

        Ok(())
    }
}

/// Load cache from disk.
pub fn load_cache(cache_dir: &Path, package_name: &str) -> ImportsCache {
    let cache_file = cache_dir.join(format!("{}.imports.bincode", package_name));

    let cache_map = if let Ok(mut file) = fs::File::open(&cache_file) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            if let Ok(decoded) = bincode::decode_from_slice::<HashMap<String, CachedImports>, _>(
                &buffer,
                bincode::config::standard(),
            ) {
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

    ImportsCache {
        cache: cache_map,
        cache_dir: cache_dir.to_path_buf(),
        package_name: package_name.to_string(),
    }
}
