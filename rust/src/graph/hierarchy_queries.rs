use crate::graph::{Graph, Module, ModuleToken, MODULE_NAMES};

impl Graph {
    pub fn get_module_by_name(&self, name: &str) -> Option<&Module> {
        let interner = MODULE_NAMES.read().unwrap();
        let name = interner.get(name)?;
        match self.modules_by_name.get_by_left(&name) {
            Some(token) => self.get_module(*token),
            None => None,
        }
    }

    pub fn get_module(&self, module: ModuleToken) -> Option<&Module> {
        self.modules.get(module)
    }

    // TODO(peter) Guarantee order?
    pub fn all_modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    pub fn get_module_parent(&self, module: ModuleToken) -> Option<&Module> {
        match self.module_parents.get(module) {
            Some(parent) => parent.map(|parent| self.get_module(parent).unwrap()),
            None => None,
        }
    }

    pub fn get_module_children(&self, module: ModuleToken) -> impl Iterator<Item = &Module> {
        let children = match self.module_children.get(module) {
            Some(children) => children
                .iter()
                .map(|child| self.get_module(*child).unwrap())
                .collect(),
            None => Vec::new(),
        };
        children.into_iter()
    }

    /// Returns an iterator over the passed modules descendants.
    ///
    /// Parent modules will be yielded before their child modules.
    pub fn get_module_descendants(&self, module: ModuleToken) -> impl Iterator<Item = &Module> {
        let mut descendants = self.get_module_children(module).collect::<Vec<_>>();
        for child in descendants.clone() {
            descendants.extend(self.get_module_descendants(child.token).collect::<Vec<_>>())
        }
        descendants.into_iter()
    }
}
