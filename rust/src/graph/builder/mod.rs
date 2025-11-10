use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::thread;

use crossbeam::channel;
use derive_new::new;
use ignore::WalkBuilder;

use crate::errors::{GrimpError, GrimpResult};
use crate::graph::Graph;
use crate::import_parsing::{ImportedObject, parse_imports_from_code};

mod cache;
use cache::{CachedImports, ImportCache, load_cache, save_cache};

mod utils;
use utils::{
    ResolvedImport, is_internal, is_package, path_to_module_name, resolve_external_module,
    resolve_internal_module, resolve_relative_import,
};

#[derive(Debug, Clone, new)]
pub struct PackageSpec {
    name: String,
    directory: PathBuf,
}

pub fn build_graph(
    package: &PackageSpec, // TODO(peter) Support multiple packages
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Option<&PathBuf>,
) -> GrimpResult<Graph> {
    // Check if package directory exists
    if !package.directory.exists() {
        return Err(GrimpError::PackageDirectoryNotFound(
            package.directory.display().to_string(),
        ));
    }

    // Load cache if available
    let mut cache = cache_dir
        .map(|dir| load_cache(dir, &package.name))
        .unwrap_or_default();

    // Create channels for communication
    // This way we can start parsing moduels while we're still discovering them.
    let (found_module_sender, found_module_receiver) = channel::bounded(1000);
    let (parsed_module_sender, parser_module_receiver) = channel::bounded(1000);
    let (error_sender, error_receiver) = channel::bounded(1);

    let mut thread_handles = Vec::new();

    // Thread 1: Discover modules
    let package_clone = package.clone();
    let handle = thread::spawn(move || {
        discover_python_modules(&package_clone, found_module_sender);
    });
    thread_handles.push(handle);

    // Thread pool: Parse imports
    let num_threads = num_threads_for_module_parsing();
    for _ in 0..num_threads {
        let receiver = found_module_receiver.clone();
        let sender = parsed_module_sender.clone();
        let error_sender = error_sender.clone();
        let cache = cache.clone();
        let handle = thread::spawn(move || {
            while let Ok(module) = receiver.recv() {
                match parse_module_imports(&module, &cache) {
                    Ok(parsed) => {
                        let _ = sender.send(parsed);
                    }
                    Err(e) => {
                        // Channel has capacity of 1, since we only care to catch one error.
                        // Drop further errors.
                        let _ = error_sender.try_send(e);
                    }
                }
            }
        });
        thread_handles.push(handle);
    }

    // Close original receivers/senders so threads know when to stop
    drop(parsed_module_sender); // Main thread will know when no more parsed modules

    // Collect parsed modules (this will continue until all parser threads finish and close their senders)
    let mut parsed_modules = Vec::new();
    while let Ok(parsed) = parser_module_receiver.recv() {
        parsed_modules.push(parsed);
    }

    // Wait for all threads to complete
    for handle in thread_handles {
        handle.join().unwrap();
    }

    // Check if any errors occurred
    if let Ok(error) = error_receiver.try_recv() {
        return Err(error);
    }

    // Update and save cache if cache_dir is set
    if let Some(cache_dir) = cache_dir {
        for parsed in &parsed_modules {
            cache.insert(
                parsed.module.path.clone(),
                CachedImports::new(parsed.module.mtime_secs, parsed.imported_objects.clone()),
            );
        }
        save_cache(&cache, cache_dir, &package.name)?;
    }

    // Resolve imports and assemble graph
    let imports_by_module = resolve_imports(
        &parsed_modules,
        include_external_packages,
        exclude_type_checking_imports,
    );

    Ok(assemble_graph(&imports_by_module, &package.name))
}

#[derive(Debug, Clone)]
struct FoundModule {
    name: String,
    path: PathBuf,
    is_package: bool,
    mtime_secs: i64,
}

#[derive(Debug)]
struct ParsedModule {
    module: FoundModule,
    imported_objects: Vec<ImportedObject>,
}

fn discover_python_modules(package: &PackageSpec, sender: channel::Sender<FoundModule>) {
    let num_threads = num_threads_for_module_discovery();
    let package_clone = package.clone();

    WalkBuilder::new(&package.directory)
        .standard_filters(false) // Don't use gitignore or other filters
        .threads(num_threads)
        .filter_entry(|entry| {
            // Allow Python files
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                return entry.path().extension().and_then(|s| s.to_str()) == Some("py");
            }

            // For directories, only descend if they contain __init__.py
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                let init_path = entry.path().join("__init__.py");
                return init_path.exists();
            }

            false
        })
        .build_parallel()
        .run(|| {
            let sender = sender.clone();
            let package = package_clone.clone();

            Box::new(move |entry| {
                use ignore::WalkState;

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return WalkState::Continue,
                };

                let path = entry.path();

                // Only process files, not directories
                if !path.is_file() {
                    return WalkState::Continue;
                }

                if let Some(module_name) = path_to_module_name(path, &package) {
                    let is_package = is_package(path);

                    // Get mtime
                    let mtime_secs = fs::metadata(path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    let found_module = FoundModule {
                        name: module_name,
                        path: path.to_owned(),
                        is_package,
                        mtime_secs,
                    };

                    // Send module as soon as we discover it
                    let _ = sender.send(found_module);
                }

                WalkState::Continue
            })
        });
}

fn parse_module_imports(module: &FoundModule, cache: &ImportCache) -> GrimpResult<ParsedModule> {
    // Check if we have a cached version with matching mtime
    if let Some(cached) = cache.get(&module.path)
        && module.mtime_secs == cached.mtime_secs()
    {
        // Cache hit - use cached imports
        return Ok(ParsedModule {
            module: module.clone(),
            imported_objects: cached.imported_objects().to_vec(),
        });
    }

    // Cache miss or file modified - parse the file
    let code = fs::read_to_string(&module.path).map_err(|e| GrimpError::FileReadError {
        path: module.path.display().to_string(),
        error: e.to_string(),
    })?;

    let imported_objects = parse_imports_from_code(&code, module.path.to_str().unwrap_or(""))?;

    Ok(ParsedModule {
        module: module.clone(),
        imported_objects,
    })
}

fn resolve_imports(
    parsed_modules: &[ParsedModule],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> HashMap<String, HashSet<ResolvedImport>> {
    let all_modules: HashSet<String> = parsed_modules
        .iter()
        .map(|module| module.module.name.clone())
        .collect();

    let mut imports_by_module = HashMap::new();
    for parsed_module in parsed_modules {
        let mut resolved_imports = HashSet::new();

        // Resolve each imported object
        for imported_object in &parsed_module.imported_objects {
            // Skip type checking imports if requested
            if exclude_type_checking_imports && imported_object.typechecking_only {
                continue;
            }

            // Resolve relative imports to absolute
            let absolute_import_name = resolve_relative_import(
                &parsed_module.module.name,
                parsed_module.module.is_package,
                &imported_object.name,
            );

            // Try to resolve as internal module first
            if let Some(internal_module) =
                resolve_internal_module(&absolute_import_name, &all_modules)
            {
                resolved_imports.insert(ResolvedImport {
                    importer: parsed_module.module.name.to_string(),
                    imported: internal_module,
                    line_number: imported_object.line_number,
                    line_contents: imported_object.line_contents.clone(),
                });
            } else if include_external_packages {
                // It's an external module and we're including them
                let external_module = resolve_external_module(&absolute_import_name);
                resolved_imports.insert(ResolvedImport {
                    importer: parsed_module.module.name.to_string(),
                    imported: external_module,
                    line_number: imported_object.line_number,
                    line_contents: imported_object.line_contents.clone(),
                });
            }
        }

        imports_by_module.insert(parsed_module.module.name.clone(), resolved_imports);
    }

    imports_by_module
}

fn assemble_graph(
    imports_by_module: &HashMap<String, HashSet<ResolvedImport>>,
    package_name: &str,
) -> Graph {
    let mut graph = Graph::default();

    // Add all modules and their imports
    for (module_name, imports) in imports_by_module {
        // Add the module itself and get its token
        let importer_token = graph.get_or_add_module(module_name).token();

        for import in imports {
            // Add the imported module
            let imported_token = if is_internal(&import.imported, package_name) {
                graph.get_or_add_module(&import.imported).token()
            } else {
                graph.get_or_add_squashed_module(&import.imported).token()
            };

            // Add the import with details
            graph.add_detailed_import(
                importer_token,
                imported_token,
                import.line_number as u32,
                &import.line_contents,
            );
        }
    }

    graph
}

/// Calculate the number of threads to use for module discovery.
/// Uses half of available parallelism, with a minimum of 1 and default of 4.
fn num_threads_for_module_discovery() -> usize {
    thread::available_parallelism()
        .map(|n| max(n.get() / 2, 1))
        .unwrap_or(4)
}

/// Calculate the number of threads to use for module parsing.
/// Uses half of available parallelism, with a minimum of 1 and default of 4.
fn num_threads_for_module_parsing() -> usize {
    thread::available_parallelism()
        .map(|n| max(n.get() / 2, 1))
        .unwrap_or(4)
}
