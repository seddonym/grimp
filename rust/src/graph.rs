use crate::hierarchy::ModuleHierarchy;
use crate::imports::{ImportDetails, ModuleImports};
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleNotPresent<'a> {
    pub module: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoSuchContainer {
    pub container: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DetailedImport<'a> {
    pub importer: &'a str,
    pub imported: &'a str,
    pub line_number: usize,
    pub line_contents: &'a str,
}

#[derive(Default, Clone)]
pub struct Graph {
    module_hierarchy: ModuleHierarchy,
    module_imports: ModuleImports,
}

impl Graph {
    pub fn add_module(&mut self, module: &str) {
        self.module_hierarchy.add_module(module).unwrap();
    }

    pub fn add_squashed_module(&mut self, module: &str) {
        let module = self.module_hierarchy.add_module(module).unwrap();
        self.module_hierarchy.mark_squashed(module).unwrap();
    }

    pub fn remove_module(&mut self, module: &str) {
        if let Some(module) = self.module_hierarchy.get_by_name(module) {
            self.module_hierarchy.remove_module(module.token()).unwrap()
            // TODO(peter) Remove imports
        }
    }

    pub fn get_modules(&self) -> FxHashSet<&str> {
        todo!()
    }

    pub fn count_imports(&self) -> usize {
        todo!()
    }

    pub fn get_import_details(&self, importer: &str, imported: &str) -> FxHashSet<DetailedImport> {
        todo!()
    }

    pub fn find_children(&self, module: &str) -> FxHashSet<&str> {
        todo!()
    }

    pub fn find_descendants(&self, module: &str) -> Result<FxHashSet<&str>, ModuleNotPresent> {
        todo!()
    }

    pub fn add_import(&mut self, importer: &str, imported: &str) {
        let importer = match self.module_hierarchy.get_by_name(importer) {
            Some(importer) => importer.token(),
            None => self.module_hierarchy.add_module(importer).unwrap(),
        };
        let imported = match self.module_hierarchy.get_by_name(imported) {
            Some(imported) => imported.token(),
            None => self.module_hierarchy.add_module(imported).unwrap(),
        };

        if self.module_imports.direct_import_exists(importer, imported) {
            return;
        }

        self.module_imports
            .add_import(importer, imported, None)
            .unwrap();
    }

    pub fn add_detailed_import(&mut self, import: &DetailedImport) {
        let importer = match self.module_hierarchy.get_by_name(import.importer) {
            Some(importer) => importer.token(),
            None => self.module_hierarchy.add_module(import.importer).unwrap(),
        };
        let imported = match self.module_hierarchy.get_by_name(import.imported) {
            Some(imported) => imported.token(),
            None => self.module_hierarchy.add_module(import.imported).unwrap(),
        };

        if self.module_imports.direct_import_exists(importer, imported) {
            return;
        }

        self.module_imports
            .add_import(
                importer,
                imported,
                Some(ImportDetails::new(import.line_number, import.line_contents)),
            )
            .unwrap();
    }

    pub fn remove_import(&mut self, importer: &str, imported: &str) {
        todo!()
    }

    // Note: this will panic if importer and imported are in the same package.
    pub fn direct_import_exists(&self, importer: &str, imported: &str, as_packages: bool) -> bool {
        todo!()
    }

    pub fn find_modules_that_directly_import(&self, imported: &str) -> FxHashSet<&str> {
        todo!()
    }

    pub fn find_modules_directly_imported_by(&self, importer: &str) -> FxHashSet<&str> {
        todo!()
    }

    pub fn find_upstream_modules(&self, module: &str, as_package: bool) -> FxHashSet<&str> {
        todo!()
    }

    pub fn find_downstream_modules(&self, module: &str, as_package: bool) -> FxHashSet<&str> {
        todo!()
    }

    pub fn find_shortest_chain(&self, importer: &str, imported: &str) -> Option<Vec<&str>> {
        todo!()
    }

    pub fn find_shortest_chains(
        &self,
        importer: &str,
        imported: &str,
        as_packages: bool,
    ) -> Result<FxHashSet<Vec<String>>, String> {
        todo!()
    }

    pub fn chain_exists(&self, importer: &str, imported: &str, as_packages: bool) -> bool {
        todo!()
    }

    // pub fn find_illegal_dependencies_for_layers(
    //     &self,
    //     levels: Vec<Level>,
    //     containers: FxHashSet<String>,
    // ) -> Result<Vec<PackageDependency>, NoSuchContainer> {
    //     todo!()
    // }

    pub fn squash_module(&mut self, module: &str) {
        todo!()
    }

    pub fn is_module_squashed(&self, module: &str) -> bool {
        todo!()
    }
}
