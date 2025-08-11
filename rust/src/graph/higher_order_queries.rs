use crate::errors::GrimpResult;
use crate::graph::{ExtendWithDescendants, Graph, ModuleToken};
use derive_new::new;
use getset::Getters;
use rayon::prelude::*;
use rustc_hash::FxHashSet;

use tap::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, new, Getters)]
pub struct Level {
    #[new(into)]
    #[getset(get = "pub")]
    layers: FxHashSet<ModuleToken>,

    #[getset(get_copy = "pub")]
    independent: bool,

    #[getset(get_copy = "pub")]
    closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, new, Getters)]
pub struct PackageDependency {
    #[getset(get = "pub")]
    importer: ModuleToken,

    #[getset(get = "pub")]
    imported: ModuleToken,

    #[new(into)]
    #[getset(get = "pub")]
    routes: Vec<Route>,
}

#[derive(Debug, Clone, PartialEq, Eq, new, Getters)]
pub struct Route {
    #[new(into)]
    #[getset(get = "pub")]
    heads: FxHashSet<ModuleToken>,

    #[new(into)]
    #[getset(get = "pub")]
    middle: Vec<ModuleToken>,

    #[new(into)]
    #[getset(get = "pub")]
    tails: FxHashSet<ModuleToken>,
}

impl Graph {
    pub fn find_illegal_dependencies_for_layers(
        &self,
        levels: &[Level],
    ) -> GrimpResult<Vec<PackageDependency>> {
        let all_layer_modules = levels
            .iter()
            .flat_map(|level| level.layers().clone())
            .flat_map(|m| m.conv::<FxHashSet<_>>().with_descendants(self))
            .collect::<FxHashSet<_>>();

        self.generate_illegal_import_permutations_for_layers(levels)
            .into_par_iter()
            .try_fold(
                Vec::new,
                |mut v: Vec<PackageDependency>, (from_package, to_package)| -> GrimpResult<_> {
                    if let Some(dep) = self.find_illegal_dependencies(
                        from_package,
                        to_package,
                        &all_layer_modules,
                    )? {
                        v.push(dep);
                    }
                    Ok(v)
                },
            )
            .try_reduce(
                Vec::new,
                |mut v: Vec<PackageDependency>, package_dependencies| {
                    v.extend(package_dependencies);
                    Ok(v)
                },
            )
    }

    /// Returns a set of tuples (importer, imported) describing the illegal
    /// import permutations for the given layers.
    fn generate_illegal_import_permutations_for_layers(
        &self,
        levels: &[Level],
    ) -> FxHashSet<(ModuleToken, ModuleToken)> {
        let mut permutations = FxHashSet::default();

        for (index, level) in levels.iter().enumerate() {
            for module in &level.layers {
                // Should not be imported by lower layers.
                for lower_level in &levels[index + 1..] {
                    for lower_module in &lower_level.layers {
                        permutations.insert((*lower_module, *module));
                    }
                }

                // Should not import siblings (if level is independent)
                if level.independent {
                    for sibling_module in &level.layers {
                        if sibling_module == module {
                            continue;
                        }
                        permutations.insert((*module, *sibling_module));
                    }
                }

                // Should not be imported by higher layers if there is a closed layer inbetween.
                let mut closed = false;
                for higher_level in levels[..index].iter().rev() {
                    if closed {
                        for higher_module in &higher_level.layers {
                            permutations.insert((*higher_module, *module));
                        }
                    }
                    closed |= higher_level.closed;
                }
            }
        }

        permutations
    }

    fn find_illegal_dependencies(
        &self,
        from_layer: ModuleToken,
        to_layer: ModuleToken,
        all_layers_modules: &FxHashSet<ModuleToken>,
    ) -> GrimpResult<Option<PackageDependency>> {
        // Shortcut the detailed implementation in the case of no chains.
        // This will be much faster!
        if !self.chain_exists(from_layer, to_layer, true)? {
            return Ok(None);
        }

        let from_layer_with_descendants = from_layer.conv::<FxHashSet<_>>().with_descendants(self);
        let to_layer_with_descendants = to_layer.conv::<FxHashSet<_>>().with_descendants(self);

        // Disallow chains via other layers.
        let excluded_modules =
            all_layers_modules - &(&from_layer_with_descendants | &to_layer_with_descendants);

        let chains = self._find_shortest_chains(
            &from_layer_with_descendants,
            &to_layer_with_descendants,
            &excluded_modules,
        )?;

        // Collect direct imports...
        let mut direct_imports = vec![];
        // ...and the middles of any indirect imports.
        let mut middles = vec![];

        for chain in chains {
            let (head, middle, tail) = self.split_chain(&chain);
            match middle {
                Some(middle) => middles.push(middle),
                None => direct_imports.push((head, tail)),
            }
        }

        // Map to routes.
        let mut routes = vec![];
        for (importer, imported) in direct_imports {
            routes.push(Route::new(importer, vec![], imported));
        }
        for middle in middles {
            let heads = from_layer_with_descendants
                .iter()
                .filter(|importer| {
                    self.direct_import_exists(**importer, *middle.first().unwrap(), false)
                        .unwrap()
                })
                .cloned()
                .collect::<FxHashSet<_>>();
            let tails = to_layer_with_descendants
                .iter()
                .filter(|imported| {
                    self.direct_import_exists(*middle.last().unwrap(), **imported, false)
                        .unwrap()
                })
                .cloned()
                .collect::<FxHashSet<_>>();
            routes.push(Route::new(heads, middle, tails));
        }

        match routes.is_empty() {
            true => Ok(None),
            false => Ok(Some(PackageDependency::new(from_layer, to_layer, routes))),
        }
    }

    fn split_chain(
        &self,
        chain: &[ModuleToken],
    ) -> (ModuleToken, Option<Vec<ModuleToken>>, ModuleToken) {
        if chain.len() == 2 {
            return (chain[0], None, chain[1]);
        }
        (
            chain[0],
            Some(chain[1..chain.len() - 1].to_vec()),
            chain[chain.len() - 1],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use rustc_hash::FxHashSet;

    #[test]
    fn test_generate_module_permutations_simple_layers() {
        let mut graph = Graph::default();

        let top_module = graph.get_or_add_module("app.top").token;
        let middle_module = graph.get_or_add_module("app.middle").token;
        let bottom_module = graph.get_or_add_module("app.bottom").token;

        let mut top_layer = FxHashSet::default();
        top_layer.insert(top_module);

        let mut middle_layer = FxHashSet::default();
        middle_layer.insert(middle_module);

        let mut bottom_layer = FxHashSet::default();
        bottom_layer.insert(bottom_module);

        let top_level = Level::new(top_layer, false, false);
        let middle_level = Level::new(middle_layer, false, false);
        let bottom_level = Level::new(bottom_layer, false, false);

        let levels = vec![top_level, middle_level, bottom_level];

        let permutations = graph.generate_illegal_import_permutations_for_layers(&levels);

        assert_eq!(
            permutations,
            FxHashSet::from_iter([
                (bottom_module, middle_module),
                (bottom_module, top_module),
                (middle_module, top_module),
            ])
        );
    }

    #[test]
    fn test_generate_module_permutations_independent_layer() {
        let mut graph = Graph::default();

        let module_a = graph.get_or_add_module("app.independent.a").token;
        let module_b = graph.get_or_add_module("app.independent.b").token;

        let mut independent_layer = FxHashSet::default();
        independent_layer.insert(module_a);
        independent_layer.insert(module_b);

        let independent_level = Level::new(independent_layer, true, false);

        let levels = vec![independent_level];

        let permutations = graph.generate_illegal_import_permutations_for_layers(&levels);

        assert_eq!(
            permutations,
            FxHashSet::from_iter([(module_a, module_b), (module_b, module_a),])
        );
    }

    #[test]
    fn test_generate_module_permutations_closed_layer() {
        let mut graph = Graph::default();

        // Create three layers with the middle one closed
        let top_module = graph.get_or_add_module("app.top").token;
        let middle_module = graph.get_or_add_module("app.middle").token;
        let bottom_module = graph.get_or_add_module("app.bottom").token;

        let mut top_layer = FxHashSet::default();
        top_layer.insert(top_module);

        let mut middle_layer = FxHashSet::default();
        middle_layer.insert(middle_module);

        let mut bottom_layer = FxHashSet::default();
        bottom_layer.insert(bottom_module);

        let top_level = Level::new(top_layer, false, false);
        let middle_level = Level::new(middle_layer, false, true); // Closed layer
        let bottom_level = Level::new(bottom_layer, false, false);

        let levels = vec![top_level, middle_level, bottom_level];

        let permutations = graph.generate_illegal_import_permutations_for_layers(&levels);

        assert_eq!(
            permutations,
            FxHashSet::from_iter([
                (bottom_module, middle_module),
                (bottom_module, top_module),
                (middle_module, top_module),
                // Top should not import Bottom due to closed middle layer
                (top_module, bottom_module),
            ])
        );
    }
}
