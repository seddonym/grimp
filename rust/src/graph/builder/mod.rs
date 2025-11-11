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

mod imports_cache;
use imports_cache::ImportsCache;

mod utils;
use utils::{
    ResolvedImport, distill_external_module, is_internal, is_package, path_to_module_name,
    resolve_internal_module, resolve_relative_import,
};

mod read_python_file;
use read_python_file::read_python_file;

#[derive(Debug, Clone, new)]
pub struct PackageSpec {
    name: String,
    directory: PathBuf,
}

pub fn build_graph(
    packages: &[PackageSpec],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Option<&PathBuf>,
) -> GrimpResult<Graph> {
    // Check all package directories exist
    for package in packages {
        if !package.directory.exists() {
            return Err(GrimpError::PackageDirectoryNotFound(
                package.directory.display().to_string(),
            ));
        }
    }
    let package_names: HashSet<String> = packages.iter().map(|p| p.name.clone()).collect();

    // Discover and parse all modules from all packages in parallel
    let parsed_modules = discover_and_parse_modules(packages, cache_dir)?;

    // Resolve imports and assemble graph
    let imports_by_module = resolve_imports(
        &parsed_modules,
        include_external_packages,
        exclude_type_checking_imports,
        &package_names,
    );

    Ok(assemble_graph(&imports_by_module, &package_names))
}

#[derive(Debug, Clone)]
struct FoundModule {
    package_name: String,
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

/// Discover and parse all Python modules in one or more packages using parallel processing.
///
/// # Concurrency Model
///
/// This function uses a pipeline architecture with three stages:
///
/// 1. **Discovery Stage** (1 thread):
///    - Walks all package directories to find Python files
///    - Sends found modules to the parsing stage via `found_module_sender`
///    - Closes the channel when complete
///
/// 2. **Parsing Stage** (N worker threads):
///    - Each thread receives modules from `found_module_receiver`
///    - Parses imports from each module (with caching)
///    - Sends parsed modules to the collection stage via `parsed_module_sender`
///    - Threads exit when `found_module_sender` is dropped (discovery complete)
///
/// 3. **Collection Stage** (main thread):
///    - Receives parsed modules from `parser_module_receiver`
///    - Stops when all parser threads exit and drop their `parsed_module_sender` clones
///
/// # Error Handling
///
/// Parse errors are sent via `error_sender`. We only capture the first error and return it.
/// Subsequent errors are dropped since we fail fast on the first error.
///
/// # Returns
///
/// Returns a vector parsed modules, or the first error encountered.
fn discover_and_parse_modules(
    packages: &[PackageSpec],
    cache_dir: Option<&PathBuf>,
) -> GrimpResult<Vec<ParsedModule>> {
    let thread_counts = calculate_thread_counts();

    thread::scope(|scope| {
        // Load cache for all packages if available
        let mut cache: Option<ImportsCache> = cache_dir.map(|dir| {
            let package_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
            ImportsCache::load(dir, &package_names)
        });

        // Create channels for the pipeline
        let (found_module_sender, found_module_receiver) = channel::bounded(1000);
        let (parsed_module_sender, parsed_module_receiver) = channel::bounded(1000);
        let (error_sender, error_receiver) = channel::bounded(1);

        // Stage 1: Discovery thread
        scope.spawn(|| {
            discover_python_modules(
                packages,
                thread_counts.module_discovery,
                found_module_sender,
            );
        });

        // Stage 2: Parser thread pool
        for _ in 0..thread_counts.module_parsing {
            let receiver = found_module_receiver.clone();
            let sender = parsed_module_sender.clone();
            let error_sender = error_sender.clone();
            let cache = cache.clone();
            scope.spawn(move || {
                while let Ok(module) = receiver.recv() {
                    match parse_module_imports(&module, cache.as_ref()) {
                        Ok(parsed) => {
                            let _ = sender.send(parsed);
                        }
                        Err(e) => {
                            let _ = error_sender.try_send(e);
                        }
                    }
                }
            });
        }

        // Close our copy of the sender so the receiver knows when all threads are done
        drop(parsed_module_sender);

        // Stage 3: Collection (in main thread)
        let mut parsed_modules = Vec::new();
        while let Ok(parsed) = parsed_module_receiver.recv() {
            parsed_modules.push(parsed);
        }

        // Check if any errors occurred
        if let Ok(error) = error_receiver.try_recv() {
            return Err(error);
        }

        // Update and save cache
        if let Some(cache) = &mut cache {
            for parsed in &parsed_modules {
                cache.set_imports(
                    parsed.module.package_name.clone(),
                    parsed.module.name.clone(),
                    parsed.module.mtime_secs,
                    parsed.imported_objects.clone(),
                );
            }
            cache.save()?;
        }

        Ok(parsed_modules)
    })
}

fn discover_python_modules(
    packages: &[PackageSpec],
    num_threads: usize,
    sender: channel::Sender<FoundModule>,
) {
    let packages: Vec<PackageSpec> = packages.to_vec();

    let mut builder = WalkBuilder::new(&packages[0].directory);
    for package in &packages[1..] {
        builder.add(&package.directory);
    }
    builder
        .standard_filters(false) // Don't use gitignore or other filters
        .hidden(true) // Ignore hidden files/directories
        .threads(num_threads)
        .filter_entry(|entry| {
            // Allow Python files (but skip files with multiple dots like dotted.module.py)
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("py") {
                    // Check if filename has multiple dots (invalid Python module names)
                    if let Some(file_name) = entry.file_name().to_str() {
                        return file_name.matches('.').count() == 1; // Only the .py extension
                    }
                }
                return false;
            }

            // For directories, only descend if they contain __init__.py
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                let init_path = entry.path().join("__init__.py");
                return init_path.exists();
            }

            false
        });

    builder.build_parallel().run(|| {
        let sender = sender.clone();
        let packages = packages.clone();

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

            // Find which package this file belongs to by checking if path starts with package directory
            let package = packages
                .iter()
                .find(|pkg| path.starts_with(&pkg.directory))
                .unwrap();

            if let Some(module_name) = path_to_module_name(path, package) {
                let is_package = is_package(path);

                // Get mtime
                let mtime_secs = fs::metadata(path)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let found_module = FoundModule {
                    package_name: package.name.clone(),
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

fn parse_module_imports(
    module: &FoundModule,
    cache: Option<&ImportsCache>,
) -> GrimpResult<ParsedModule> {
    // Check if we have a cached version with matching mtime
    if let Some(cache) = cache
        && let Some(imported_objects) =
            cache.get_imports(&module.package_name, &module.name, module.mtime_secs)
    {
        // Cache hit - use cached imports
        return Ok(ParsedModule {
            module: module.clone(),
            imported_objects,
        });
    }

    // Cache miss or file modified - parse the file
    let code = read_python_file(&module.path)?;

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
    packages: &HashSet<String>,
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
                // Try to resolve as an external module
                if let Some(external_module) =
                    distill_external_module(&absolute_import_name, packages)
                {
                    resolved_imports.insert(ResolvedImport {
                        importer: parsed_module.module.name.to_string(),
                        imported: external_module,
                        line_number: imported_object.line_number,
                        line_contents: imported_object.line_contents.clone(),
                    });
                }
            }
        }

        imports_by_module.insert(parsed_module.module.name.clone(), resolved_imports);
    }

    imports_by_module
}

fn assemble_graph(
    imports_by_module: &HashMap<String, HashSet<ResolvedImport>>,
    package_names: &HashSet<String>,
) -> Graph {
    let mut graph = Graph::default();

    // Add all modules and their imports
    for (module_name, imports) in imports_by_module {
        // Add the module itself and get its token
        let importer_token = graph.get_or_add_module(module_name).token();

        for import in imports {
            // Add the imported module
            let imported_token = if is_internal(&import.imported, package_names) {
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

/// Thread counts for parallel processing stages.
struct ThreadCounts {
    module_discovery: usize,
    module_parsing: usize,
}

/// Calculate the number of threads to use for parallel operations.
/// Uses 2/3 of available CPUs for both module discovery and parsing.
/// Since both module discovery and parsing involve some IO, it makes sense to
/// use slighly more threads than the available CPUs.
fn calculate_thread_counts() -> ThreadCounts {
    let num_threads = thread::available_parallelism()
        .map(|n| max((2 * n.get()) / 3, 1))
        .unwrap_or(4);
    ThreadCounts {
        module_discovery: num_threads,
        module_parsing: num_threads,
    }
}
