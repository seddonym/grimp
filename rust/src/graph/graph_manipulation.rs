use crate::graph::{
    Graph, ImportDetails, Module, ModuleIterator, ModuleToken, IMPORT_LINE_CONTENTS, MODULE_NAMES,
};
use rustc_hash::FxHashSet;
use slotmap::secondary::Entry;

impl Graph {
    /// `foo.bar.baz => [foo.bar.baz, foo.bar, foo]`
    pub(crate) fn module_name_to_self_and_ancestors(&self, name: &str) -> Vec<String> {
        let mut names = vec![name.to_owned()];
        while let Some(parent_name) = parent_name(names.last().unwrap()) {
            names.push(parent_name);
        }
        names
    }

    pub fn get_or_add_module(&mut self, name: &str) -> &Module {
        if let Some(module) = self.get_module_by_name(name) {
            let module = self.modules.get_mut(module.token).unwrap();
            module.is_invisible = false;
            return module;
        }

        let mut ancestor_names = self.module_name_to_self_and_ancestors(name);

        {
            let mut interner = MODULE_NAMES.write().unwrap();
            let mut parent: Option<ModuleToken> = None;
            while let Some(name) = ancestor_names.pop() {
                let name = interner.get_or_intern(name);
                if let Some(module) = self.modules_by_name.get_by_left(&name) {
                    parent = Some(*module)
                } else {
                    let module = self.modules.insert_with_key(|token| Module {
                        token,
                        interned_name: name,
                        is_invisible: !ancestor_names.is_empty(),
                        is_squashed: false,
                    });
                    self.modules_by_name.insert(name, module);
                    self.module_parents.insert(module, parent);
                    self.module_children.insert(module, FxHashSet::default());
                    self.imports.insert(module, FxHashSet::default());
                    self.reverse_imports.insert(module, FxHashSet::default());
                    if let Some(parent) = parent {
                        self.module_children[parent].insert(module);
                    }
                    parent = Some(module)
                }
            }
        }

        self.get_module_by_name(name).unwrap()
    }

    pub fn get_or_add_squashed_module(&mut self, module: &str) -> &Module {
        let module = self.get_or_add_module(module).token();
        self.mark_module_squashed(module);
        self.get_module(module).unwrap()
    }

    fn mark_module_squashed(&mut self, module: ModuleToken) {
        let module = self.modules.get_mut(module).unwrap();
        if !self.module_children[module.token].is_empty() {
            panic!("cannot mark a module with children as squashed")
        }
        module.is_squashed = true;
    }

    pub fn remove_module(&mut self, module: ModuleToken) {
        let module = self.get_module(module);
        if module.is_none() {
            return;
        }
        let module = module.unwrap().token();

        // TODO(peter) Remove children automatically here, or raise an error?
        if !self.module_children[module].is_empty() {
            for child in self.module_children[module].clone() {
                self.remove_module(child);
            }
        }

        // Update hierarchy.
        if let Some(parent) = self.module_parents[module] {
            self.module_children[parent].remove(&module);
        }
        self.modules_by_name.remove_by_right(&module);
        self.modules.remove(module);
        self.module_parents.remove(module);
        self.module_children.remove(module);

        // Update imports.
        for imported in self.modules_directly_imported_by(module).clone() {
            self.remove_import(module, imported);
        }
        for importer in self.modules_that_directly_import(module).clone() {
            self.remove_import(importer, module);
        }
        self.imports.remove(module);
        self.reverse_imports.remove(module);
    }

    pub fn add_import(&mut self, importer: ModuleToken, imported: ModuleToken) {
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
    }

    pub fn add_detailed_import(
        &mut self,
        importer: ModuleToken,
        imported: ModuleToken,
        line_number: u32,
        line_contents: &str,
    ) {
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
        {
            let mut interner = IMPORT_LINE_CONTENTS.write().unwrap();
            let line_contents = interner.get_or_intern(line_contents);
            self.import_details
                .entry((importer, imported))
                .or_default()
                .insert(ImportDetails::new(line_number, line_contents));
        }
    }

    pub fn remove_import(&mut self, importer: ModuleToken, imported: ModuleToken) {
        match self.imports.entry(importer).unwrap() {
            Entry::Occupied(mut entry) => {
                entry.get_mut().remove(&imported);
            }
            Entry::Vacant(_) => {}
        };
        match self.reverse_imports.entry(imported).unwrap() {
            Entry::Occupied(mut entry) => {
                entry.get_mut().remove(&importer);
            }
            Entry::Vacant(_) => {}
        };
        self.import_details.remove(&(importer, imported));
    }

    pub fn squash_module(&mut self, module: ModuleToken) {
        // Get descendants and their imports.
        let descendants: FxHashSet<_> = self.get_module_descendants(module).tokens().collect();

        let modules_imported_by_descendants: FxHashSet<_> = descendants
            .iter()
            .flat_map(|descendant| {
                self.modules_directly_imported_by(*descendant)
                    .iter()
                    .cloned()
            })
            .collect();
        let modules_that_import_descendants: FxHashSet<_> = descendants
            .iter()
            .flat_map(|descendant| {
                self.modules_that_directly_import(*descendant)
                    .iter()
                    .cloned()
            })
            .collect();

        // Add descendants and imports to parent module.
        for imported in modules_imported_by_descendants {
            self.add_import(module, imported);
        }

        for importer in modules_that_import_descendants {
            self.add_import(importer, module);
        }

        // Remove any descendants.
        for descendant in descendants {
            self.remove_module(descendant);
        }

        self.mark_module_squashed(module);
    }
}

fn parent_name(name: &str) -> Option<String> {
    name.rsplit_once(".").map(|(base, _)| base.to_owned())
}
