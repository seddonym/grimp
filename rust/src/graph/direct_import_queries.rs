use crate::errors::{GrimpError, GrimpResult};
use crate::graph::{
    ExtendWithDescendants, Graph, ImportDetails, ModuleToken, EMPTY_IMPORT_DETAILS,
    EMPTY_MODULE_TOKENS, MODULE_NAMES,
};
use crate::module_expressions::ModuleExpression;
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

    pub fn find_matching_direct_imports(
        &self,
        importer_expression: &ModuleExpression,
        imported_expression: &ModuleExpression,
    ) -> FxHashSet<(ModuleToken, ModuleToken)> {
        let interner = MODULE_NAMES.read().unwrap();
        self.imports
            .iter()
            .flat_map(|(importer, imports)| {
                imports
                    .iter()
                    .cloned()
                    .map(move |imported| (importer, imported))
            })
            .filter(|(importer, imported)| {
                let importer = self.get_module(*importer).unwrap();
                let importer = interner.resolve(importer.interned_name).unwrap();
                let imported = self.get_module(*imported).unwrap();
                let imported = interner.resolve(imported.interned_name).unwrap();
                importer_expression.is_match(importer) && imported_expression.is_match(imported)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_matching_direct_imports() {
        let mut graph = Graph::default();

        let _ = graph.get_or_add_module("pkg.animals").token;
        let dog = graph.get_or_add_module("pkg.animals.dog").token;
        let cat = graph.get_or_add_module("pkg.animals.cat").token;
        let _ = graph.get_or_add_module("pkg.colors").token;
        let golden = graph.get_or_add_module("pkg.colors.golden").token;
        let ginger = graph.get_or_add_module("pkg.colors.ginger").token;

        graph.add_import(dog, golden);
        graph.add_import(cat, ginger);

        let importer_expression = "pkg.animals.*".parse().unwrap();
        let imported_expression = "pkg.colors.*".parse().unwrap();
        let matching_imports =
            graph.find_matching_direct_imports(&importer_expression, &imported_expression);

        assert_eq!(
            matching_imports,
            FxHashSet::from_iter([(dog, golden), (cat, ginger)])
        );
    }
}
