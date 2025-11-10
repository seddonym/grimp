use std::collections::HashMap;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use bincode::{Decode, Encode};

use crate::errors::{GrimpError, GrimpResult};
use crate::import_parsing::ImportedObject;

#[derive(Debug, Clone, Encode, Decode)]
pub struct CachedImports {
    mtime_secs: i64,
    imported_objects: Vec<ImportedObject>,
}

impl CachedImports {
    pub fn new(mtime_secs: i64, imported_objects: Vec<ImportedObject>) -> Self {
        Self {
            mtime_secs,
            imported_objects,
        }
    }

    pub fn mtime_secs(&self) -> i64 {
        self.mtime_secs
    }

    pub fn imported_objects(&self) -> &[ImportedObject] {
        &self.imported_objects
    }
}

pub type ImportCache = HashMap<PathBuf, CachedImports>;

pub fn load_cache(cache_dir: &Path, package_name: &str) -> ImportCache {
    let cache_file = cache_dir.join(format!("{}.imports.bincode", package_name));

    if let Ok(mut file) = fs::File::open(&cache_file) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok()
            && let Ok(cache) =
                bincode::decode_from_slice::<ImportCache, _>(&buffer, bincode::config::standard())
        {
            return cache.0;
        }
    }

    HashMap::new()
}

pub fn save_cache(cache: &ImportCache, cache_dir: &Path, package_name: &str) -> GrimpResult<()> {
    fs::create_dir_all(cache_dir).map_err(|e| GrimpError::CacheWriteError {
        path: cache_dir.display().to_string(),
        error: e.to_string(),
    })?;

    let cache_file = cache_dir.join(format!("{}.imports.bincode", package_name));

    let encoded = bincode::encode_to_vec(cache, bincode::config::standard()).map_err(|e| {
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
