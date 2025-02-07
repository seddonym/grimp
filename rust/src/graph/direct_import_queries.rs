use crate::errors::{GrimpError, GrimpResult};
use crate::graph::{
    ExtendWithDescendants, Graph, ImportDetails, ModuleToken, EMPTY_IMPORT_DETAILS,
    EMPTY_MODULE_TOKENS,
};
use rustc_hash::FxHashSet;

impl Graph {
    pub fn count_imports(&self) -> usize {
        self.imports.values().map(|imports| imports.len()).sum()
    }

    pub fn direct_import_exists(
        &self,
        importer: ModuleToken,
        imported: ModuleToken,
        as_packages: bool,
    ) -> GrimpResult<bool> {
        let mut importer: FxHashSet<_> = importer.into();
        let mut imported: FxHashSet<_> = imported.into();
        if as_packages {
            importer.extend_with_descendants(self);
            imported.extend_with_descendants(self);
            if !(&importer & &imported).is_empty() {
                return Err(GrimpError::SharedDescendants);
            }
        }

        let direct_imports = importer
            .iter()
            .flat_map(|module| self.imports.get(*module).unwrap().iter().cloned())
            .collect::<FxHashSet<ModuleToken>>();

        Ok(!(&direct_imports & &imported).is_empty())
    }

    pub fn modules_directly_imported_by(&self, importer: ModuleToken) -> &FxHashSet<ModuleToken> {
        self.imports.get(importer).unwrap_or(&EMPTY_MODULE_TOKENS)
    }

    pub fn modules_that_directly_import(&self, imported: ModuleToken) -> &FxHashSet<ModuleToken> {
        self.reverse_imports
            .get(imported)
            .unwrap_or(&EMPTY_MODULE_TOKENS)
    }

    pub fn get_import_details(
        &self,
        importer: ModuleToken,
        imported: ModuleToken,
    ) -> &FxHashSet<ImportDetails> {
        match self.import_details.get(&(importer, imported)) {
            Some(import_details) => import_details,
            None => &EMPTY_IMPORT_DETAILS,
        }
    }
}
