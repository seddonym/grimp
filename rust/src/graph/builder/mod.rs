use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use derive_new::new;
use ignore::WalkBuilder;

use crate::graph::Graph;
use crate::import_parsing::{ImportedObject, parse_imports_from_code};

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

#[derive(Debug, Clone)]
pub struct GraphBuilder {
    package: PackageSpec, // TODO(peter) Support multiple packages
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    // cache_dir: Option<PathBuf>  // TODO(peter)
}

impl GraphBuilder {
    pub fn new(package: PackageSpec) -> Self {
        GraphBuilder {
            package,
            include_external_packages: false,
            exclude_type_checking_imports: false,
        }
    }

    pub fn include_external_packages(mut self, yes: bool) -> Self {
        self.include_external_packages = yes;
        self
    }

    pub fn exclude_type_checking_imports(mut self, yes: bool) -> Self {
        self.exclude_type_checking_imports = yes;
        self
    }

    pub fn build(&self) -> Graph {
        let modules = discover_python_modules(&self.package);

        let parsed_modules = parse_imports(&modules);

        let imports_by_module = resolve_imports(
            &parsed_modules,
            self.include_external_packages,
            self.exclude_type_checking_imports,
        );

        assemble_graph(&imports_by_module, &self.package.name)
    }
}

#[derive(Debug, Clone)]
struct FoundModule {
    name: String,
    path: PathBuf,
    is_package: bool,
}

#[derive(Debug)]
struct ParsedModule {
    module: FoundModule,
    imported_objects: Vec<ImportedObject>,
}

fn discover_python_modules(package: &PackageSpec) -> Vec<FoundModule> {
    let mut modules = Vec::new();

    let walker = WalkBuilder::new(&package.directory)
        .standard_filters(false) // Don't use gitignore or other filters
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
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if let Some(module_name) = path_to_module_name(path, package) {
            let is_package = is_package(path);
            modules.push(FoundModule {
                name: module_name,
                path: path.to_owned(),
                is_package,
            });
        }
    }

    modules
}

fn parse_imports(modules: &[FoundModule]) -> Vec<ParsedModule> {
    let mut parsed_modules = Vec::new();

    for module in modules {
        // Read the file
        if let Ok(code) = fs::read_to_string(&module.path) {
            // Parse imports
            if let Ok(imported_objects) =
                parse_imports_from_code(&code, module.path.to_str().unwrap_or(""))
            {
                parsed_modules.push(ParsedModule {
                    module: module.clone(),
                    imported_objects,
                });
            }
        }
    }

    parsed_modules
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
