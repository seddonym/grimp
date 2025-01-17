use crate::hierarchy::ModuleToken;
use anyhow::Result;
use derive_new::new;
use getset::{CopyGetters, Getters};
use slotmap::SecondaryMap;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, new, Getters, CopyGetters)]
pub struct ImportDetails {
    #[getset(get_copy = "pub")]
    line_number: usize,

    #[new(into)]
    #[getset(get = "pub")]
    line_contents: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModuleImports {
    imports: SecondaryMap<ModuleToken, HashSet<ModuleToken>>,
    reverse_imports: SecondaryMap<ModuleToken, HashSet<ModuleToken>>,
    import_details: HashMap<(ModuleToken, ModuleToken), ImportDetails>,
}

impl ModuleImports {
    pub fn direct_import_exists(&self, importer: ModuleToken, imported: ModuleToken) -> bool {
        match self.imports.get(importer) {
            Some(imports) => imports.contains(&imported),
            None => false,
        }
    }

    pub fn add_import(
        &mut self,
        importer: ModuleToken,
        imported: ModuleToken,
        details: Option<ImportDetails>,
    ) -> Result<()> {
        self.imports
            .entry(importer)
            .unwrap()
            .or_default()
            .insert(imported);
        self.reverse_imports
            .entry(imported)
            .unwrap()
            .or_default()
            .insert(importer);
        if details.is_some() {
            self.import_details
                .insert((importer, imported), details.unwrap());
        }
        Ok(())
    }
}
