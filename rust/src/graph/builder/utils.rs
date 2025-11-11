use std::collections::HashSet;
use std::path::Path;

use crate::graph::builder::PackageSpec;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResolvedImport {
    pub importer: String,
    pub imported: String,
    pub line_number: usize,
    pub line_contents: String,
}

/// Check if a module filename represents a package (i.e., __init__.py)
pub fn is_package(module_path: &Path) -> bool {
    module_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "__init__.py")
        .unwrap_or(false)
}

/// Check if a module is a descendant of another module.
pub fn is_descendant(module_name: &str, potential_ancestor: &str) -> bool {
    module_name.starts_with(&format!("{}.", potential_ancestor))
}

/// Check if module is internal to any of the given packages
pub fn is_internal<'a>(module_name: &str, packages: impl IntoIterator<Item = &'a String>) -> bool {
    packages
        .into_iter()
        .any(|pkg| module_name == pkg || is_descendant(module_name, pkg))
}

/// Convert module path to module name
pub fn path_to_module_name(module_path: &Path, package: &PackageSpec) -> Option<String> {
    let relative_path = module_path.strip_prefix(&package.directory).ok()?;

    let mut components: Vec<String> = vec![package.name.clone()];
    for component in relative_path.iter() {
        let component_str = component.to_str()?;
        if component_str == "__init__.py" {
            // This is a package, don't add __init__
            break;
        } else if component_str.ends_with(".py") {
            // Strip .py extension
            components.push(component_str.strip_suffix(".py")?.to_string());
        } else {
            // Directory component
            components.push(component_str.to_string());
        }
    }

    Some(components.join("."))
}

/// Convert a relative import to an absolute import name
pub fn resolve_relative_import(
    module_name: &str,
    is_package: bool,
    imported_object_name: &str,
) -> String {
    let leading_dots_count = imported_object_name
        .chars()
        .take_while(|&c| c == '.')
        .count();

    if leading_dots_count == 0 {
        return imported_object_name.to_string();
    }

    let imported_object_name_base = if is_package {
        if leading_dots_count == 1 {
            module_name.to_string()
        } else {
            let parts: Vec<&str> = module_name.split('.').collect();
            parts[0..parts.len() - leading_dots_count + 1].join(".")
        }
    } else {
        let parts: Vec<&str> = module_name.split('.').collect();
        parts[0..parts.len() - leading_dots_count].join(".")
    };

    format!(
        "{}.{}",
        imported_object_name_base,
        &imported_object_name[leading_dots_count..]
    )
}

/// Resolve an imported object name to an internal module
pub fn resolve_internal_module(
    imported_object_name: &str,
    all_modules: &HashSet<String>,
) -> Option<String> {
    let candidate_module = imported_object_name.to_string();

    if all_modules.contains(&candidate_module) {
        return Some(candidate_module);
    }

    // Check if parent module exists
    if let Some((parent, _)) = imported_object_name.rsplit_once('.')
        && all_modules.contains(parent)
    {
        return Some(parent.to_string());
    }

    None
}

/// Given a module that we already know is external, turn it into a module to add to the graph.
///
/// The 'distillation' process involves removing any unwanted subpackages. For example,
/// django.models.db should be turned into simply django.
///
/// The process is more complex for potential namespace packages, as it's not possible to
/// determine the portion package simply from name. Rather than adding the overhead of a
/// filesystem read, we just get the shallowest component that does not clash with an internal
/// module namespace. Take, for example, foo.blue.alpha.one. If one of the found
/// packages is foo.blue.beta, the module will be distilled to foo.blue.alpha.
/// Alternatively, if the found package is foo.green, the distilled module will
/// be foo.blue.
///
/// Returns None if the module is a parent of one of the internal packages (doesn't make sense,
/// probably an import of a namespace package).
pub fn distill_external_module(
    module_name: &str,
    found_package_names: &HashSet<String>,
) -> Option<String> {
    for found_package in found_package_names {
        // If it's a module that is a parent of the package, return None
        // as it doesn't make sense and is probably an import of a namespace package.
        if is_descendant(found_package, module_name) {
            return None;
        }
    }

    let module_root = module_name.split('.').next().unwrap();

    // If it shares a namespace with an internal module, get the shallowest component that does
    // not clash with an internal module namespace.
    let mut candidate_portions: Vec<String> = Vec::new();
    let mut sorted_found_packages: Vec<&String> = found_package_names.iter().collect();
    sorted_found_packages.sort();
    sorted_found_packages.reverse();

    for found_package in sorted_found_packages {
        if is_descendant(found_package, module_root) {
            let mut internal_components: Vec<&str> = found_package.split('.').collect();
            let mut external_components: Vec<&str> = module_name.split('.').collect();
            let mut external_namespace_components: Vec<&str> = vec![];
            while external_components[0] == internal_components[0] {
                external_namespace_components.push(external_components.remove(0));
                internal_components.remove(0);
            }
            external_namespace_components.push(external_components[0]);
            candidate_portions.push(external_namespace_components.join("."));
        }
    }

    if !candidate_portions.is_empty() {
        // If multiple internal modules share a namespace with this module, use the deepest one
        // as we know that that will be a namespace too.
        candidate_portions.sort_by_key(|portion| portion.split('.').count());
        Some(candidate_portions.last().unwrap().clone())
    } else {
        Some(module_root.to_string())
    }
}
