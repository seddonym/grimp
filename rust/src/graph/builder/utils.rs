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

/// Check if module is internal
pub fn is_internal(module_name: &str, package: &str) -> bool {
    if module_name == package || module_name.starts_with(&format!("{}.", package)) {
        return true;
    }
    false
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

/// Get external module name
pub fn resolve_external_module(module_name: &str) -> String {
    // For simplicity, just return the root module for external imports
    // This matches the basic behavior from _distill_external_module
    module_name
        .split('.')
        .next()
        .unwrap_or(module_name)
        .to_string()
}
