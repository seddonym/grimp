use crate::graph::{Graph, MODULE_NAMES, Module, ModuleIterator, ModuleToken};
use crate::module_expressions::ModuleExpression;
use rustc_hash::FxHashSet;

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
    pub fn all_modules(&self) -> impl ModuleIterator {
        self.modules.values()
    }

    pub fn get_module_parent(&self, module: ModuleToken) -> Option<&Module> {
        match self.module_parents.get(module) {
            Some(parent) => parent.map(|parent| self.get_module(parent).unwrap()),
            None => None,
        }
    }

    pub fn get_module_children(&self, module: ModuleToken) -> impl ModuleIterator {
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
    pub fn get_module_descendants(&self, module: ModuleToken) -> impl ModuleIterator {
        let mut descendants = self.get_module_children(module).collect::<Vec<_>>();
        for child in descendants.clone() {
            descendants.extend(self.get_module_descendants(child.token).collect::<Vec<_>>())
        }
        descendants.into_iter()
    }

    pub fn find_matching_modules(
        &self,
        expression: &ModuleExpression,
    ) -> impl ModuleIterator + use<'_> {
        let interner = MODULE_NAMES.read().unwrap();
        let modules: FxHashSet<_> = self
            .modules
            .values()
            .filter(|m| expression.is_match(interner.resolve(m.interned_name).unwrap()))
            .collect();
        modules.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ModuleIterator;
    use derive_new::new;
    use parameterized::parameterized;

    #[derive(Debug, new)]
    struct FindMatchingModulesTestCase<'a> {
        expression: &'a str,
        #[new(into)]
        expected_matching_modules: Vec<&'a str>,
    }

    #[parameterized(case = {
        FindMatchingModulesTestCase::new("foo", ["foo"]),
        FindMatchingModulesTestCase::new("foo.*", ["foo.bar"]),
        FindMatchingModulesTestCase::new("foo.**", ["foo.bar", "foo.bar.baz"]),
    })]
    fn test_find_matching_modules(case: FindMatchingModulesTestCase) {
        // The test here is just a sense check - we don't need to check again here how all the
        // different types of module expressions work since that is already tested
        // thoroughly elsewhere.
        let mut graph = Graph::default();
        graph.get_or_add_module("foo");
        graph.get_or_add_module("foo.bar");
        graph.get_or_add_module("foo.bar.baz");

        let expression = case.expression.parse().unwrap();

        let matching_modules: FxHashSet<_> =
            graph.find_matching_modules(&expression).tokens().collect();

        assert_eq!(
            matching_modules,
            case.expected_matching_modules
                .into_iter()
                .map(|module| graph.get_module_by_name(module).unwrap().token)
                .collect()
        );
    }
}
