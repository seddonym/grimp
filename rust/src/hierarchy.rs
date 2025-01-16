use anyhow::{bail, Context, Result};
use bimap::BiMap;
use getset::{CopyGetters, Getters};
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::collections::HashSet;

new_key_type! { pub struct ModuleToken; }

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Getters, CopyGetters)]
pub struct Module {
    #[getset(get_copy = "pub")]
    token: ModuleToken,

    #[getset(get = "pub")]
    name: String,

    // Invisible modules exist in the hierarchy but haven't been explicitly added to the graph.
    #[getset(get_copy = "pub")]
    is_invisible: bool,

    #[getset(get_copy = "pub")]
    is_squashed: bool,
}

impl From<Module> for ModuleToken {
    fn from(value: Module) -> Self {
        value.token
    }
}

#[derive(Debug, Clone, Default)]
pub struct ModuleHierarchy {
    modules_by_name: BiMap<String, ModuleToken>,
    modules: SlotMap<ModuleToken, Module>,
    module_parents: SecondaryMap<ModuleToken, Option<ModuleToken>>,
    module_children: SecondaryMap<ModuleToken, HashSet<ModuleToken>>,
}

impl ModuleHierarchy {
    pub fn add_module(&mut self, name: &str) -> Result<ModuleToken> {
        if let Some(module) = self.get_by_name(name) {
            let module = self.modules.get_mut(module.token).unwrap();
            module.is_invisible = false;
            return Ok(module.token);
        }

        // foo.bar.baz => [foo.bar.baz, foo.bar, foo]
        let mut names = {
            let mut names = vec![name.to_owned()];
            while let Some(parent_name) = parent_name(names.last().unwrap()) {
                names.push(parent_name);
            }
            names
        };

        let mut parent: Option<ModuleToken> = None;
        while let Some(name) = names.pop() {
            if let Some(module) = self.modules_by_name.get_by_left(&name) {
                parent = Some(*module)
            } else {
                let module = self.modules.insert_with_key(|token| Module {
                    token,
                    name: name.clone(),
                    is_invisible: !names.is_empty(),
                    is_squashed: false,
                });
                self.modules_by_name.insert(name, module);
                self.module_parents.insert(module, parent);
                self.module_children.insert(module, HashSet::new());
                if let Some(parent) = parent {
                    self.module_children[parent].insert(module);
                }
                parent = Some(module)
            }
        }

        Ok(*self.modules_by_name.get_by_left(name).unwrap())
    }

    // TODO(peter) Replace with method to actually squash the module?
    pub fn mark_squashed(&mut self, module: ModuleToken) -> Result<()> {
        let module = self
            .modules
            .get_mut(module)
            .context("Module does not exist")?;
        if self.module_children[module.token].is_empty() {
            bail!("Cannot mark a module with children as squashed")
        }
        module.is_squashed = true;
        Ok(())
    }

    pub fn remove_module(&mut self, module: ModuleToken) -> Result<()> {
        if !self.modules.contains_key(module) {
            return Ok(());
        }

        if !self.module_children[module].is_empty() {
            bail!("Cannot remove a module that has children")
        }

        if let Some(parent) = self.module_parents[module] {
            self.module_children[parent].remove(&module);
        }

        self.modules_by_name.remove_by_right(&module);
        self.modules.remove(module);
        self.module_parents.remove(module);
        self.module_children.remove(module);

        Ok(())
    }

    pub fn get(&self, module: ModuleToken) -> Option<&Module> {
        self.modules.get(module)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Module> {
        match self.modules_by_name.get_by_left(name) {
            Some(token) => self.get(*token),
            None => None,
        }
    }

    pub fn get_parent(&self, module: ModuleToken) -> Option<&Module> {
        match self.module_parents.get(module) {
            Some(parent) => parent.map(|parent| self.get(parent).unwrap()),
            None => None,
        }
    }

    pub fn get_children(&self, module: ModuleToken) -> impl Iterator<Item = &Module> {
        let children = match self.module_children.get(module) {
            Some(children) => children
                .iter()
                .map(|child| self.get(*child).unwrap())
                .collect(),
            None => Vec::new(),
        };
        children.into_iter()
    }

    /// Returns an iterator over the passed modules descendants.
    ///
    /// Parent modules will be yielded before their child modules.
    pub fn get_descendants(&self, module: ModuleToken) -> impl Iterator<Item = &Module> {
        let mut descendants = self.get_children(module).collect::<Vec<_>>();
        for child in descendants.clone() {
            descendants.extend(self.get_descendants(child.token).collect::<Vec<_>>())
        }
        descendants.into_iter()
    }
}

impl From<ModuleToken> for Vec<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        vec![value]
    }
}

impl From<ModuleToken> for HashSet<ModuleToken> {
    fn from(value: ModuleToken) -> Self {
        HashSet::from([value])
    }
}

pub trait ExtendWithDescendants:
    Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>
{
    /// Extend this collection of module tokens with all descendant items.
    fn extend_with_descendants(&mut self, hierarchy: &ModuleHierarchy) {
        for item in self.clone().into_iter() {
            let descendants = hierarchy.get_descendants(item).map(|item| item.token());
            self.extend(descendants);
        }
    }

    /// Extend this collection of module tokens with all descendant items.
    fn with_descendants(mut self, hierarchy: &ModuleHierarchy) -> Self {
        self.extend_with_descendants(hierarchy);
        self
    }
}

impl<T: Sized + Clone + IntoIterator<Item = ModuleToken> + Extend<ModuleToken>>
    ExtendWithDescendants for T
{
}

fn parent_name(name: &str) -> Option<String> {
    match name.rsplit_once(".") {
        Some((base, _)) => Some(base.to_owned()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy() -> Result<()> {
        let mut hierarchy = ModuleHierarchy::default();

        let _ = hierarchy.add_module("foo.bar")?;
        let _ = hierarchy.add_module("foo.baz")?;

        let foo = hierarchy.get_by_name("foo").unwrap().token;
        let foo_bar = hierarchy.get_by_name("foo.bar").unwrap().token;
        let foo_baz = hierarchy.get_by_name("foo.baz").unwrap().token;

        assert_eq!(hierarchy.modules_by_name.len(), 3);
        assert_eq!(hierarchy.modules.len(), 3);
        assert_eq!(hierarchy.module_parents.len(), 3);
        assert_eq!(hierarchy.module_children.len(), 3);

        assert_eq!(
            hierarchy
                .get_children(foo)
                .map(|child| child.token)
                .collect::<HashSet<_>>(),
            HashSet::from([foo_bar, foo_baz]),
        );

        assert_eq!(
            hierarchy.get_parent(foo_bar).map(|parent| parent.token),
            Some(foo),
        );
        assert_eq!(
            hierarchy.get_parent(foo_baz).map(|parent| parent.token),
            Some(foo),
        );

        assert_eq!(hierarchy.get(foo).unwrap().is_invisible, true);
        assert_eq!(hierarchy.get(foo).unwrap().is_squashed, false);

        assert_eq!(hierarchy.get(foo_bar).unwrap().is_invisible, false);
        assert_eq!(hierarchy.get(foo_bar).unwrap().is_squashed, false);

        let _ = hierarchy.add_module("foo")?;
        assert_eq!(hierarchy.get(foo).unwrap().is_invisible, false);

        Ok(())
    }

    #[test]
    fn test_get_descendants() -> Result<()> {
        let mut hierarchy = ModuleHierarchy::default();

        let _ = hierarchy.add_module("foo.bar.baz.bax")?;

        let foo = hierarchy.get_by_name("foo").unwrap().token;
        let foo_bar = hierarchy.get_by_name("foo.bar").unwrap().token;
        let foo_bar_baz = hierarchy.get_by_name("foo.bar.baz").unwrap().token;
        let foo_bar_baz_bax = hierarchy.get_by_name("foo.bar.baz.bax").unwrap().token;

        assert_eq!(
            hierarchy
                .get_children(foo)
                .map(|child| child.token)
                .collect::<Vec<_>>(),
            vec![foo_bar],
        );

        // Collect into Vec<_> here rather than HashSet<_> so that we can check ordering.
        assert_eq!(
            hierarchy
                .get_descendants(foo)
                .map(|child| child.token)
                .collect::<Vec<_>>(),
            vec![foo_bar, foo_bar_baz, foo_bar_baz_bax],
        );

        Ok(())
    }

    #[test]
    fn test_extend_with_descendants() -> Result<()> {
        let mut hierarchy = ModuleHierarchy::default();

        let _ = hierarchy.add_module("foo.bar.baz.bax")?;

        let foo = hierarchy.get_by_name("foo").unwrap().token;
        let foo_bar = hierarchy.get_by_name("foo.bar").unwrap().token;
        let foo_bar_baz = hierarchy.get_by_name("foo.bar.baz").unwrap().token;
        let foo_bar_baz_bax = hierarchy.get_by_name("foo.bar.baz.bax").unwrap().token;

        let mut modules: Vec<_> = foo.into();
        modules.extend_with_descendants(&hierarchy);
        assert_eq!(modules, vec![foo, foo_bar, foo_bar_baz, foo_bar_baz_bax]);

        Ok(())
    }
}
