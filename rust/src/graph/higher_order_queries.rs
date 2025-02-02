use crate::errors::GrimpResult;
use crate::graph::{ExtendWithDescendants, Graph, ModuleToken};
use derive_new::new;
use getset::Getters;
use itertools::Itertools;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use tap::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, new, Getters)]
pub struct Level {
    #[new(into)]
    #[getset(get = "pub")]
    layers: FxHashSet<ModuleToken>,

    #[getset(get_copy = "pub")]
    independent: bool,
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

        self.generate_module_permutations(levels)
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

    fn generate_module_permutations(&self, levels: &[Level]) -> Vec<(ModuleToken, ModuleToken)> {
        let mut permutations = vec![];

        for (index, level) in levels.iter().enumerate() {
            for module in &level.layers {
                // Should not be imported by lower layers.
                for lower_level in &levels[index + 1..] {
                    for lower_module in &lower_level.layers {
                        permutations.push((*lower_module, *module));
                    }
                }

                // Should not import siblings (if level is independent)
                if level.independent {
                    for sibling_module in &level.layers {
                        if sibling_module == module {
                            continue;
                        }
                        permutations.push((*module, *sibling_module));
                    }
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

        // Disallow chains via these imports.
        // We'll add chains to this set as we discover them.
        let mut excluded_imports = FxHashMap::default();

        // Collect direct imports...
        let mut direct_imports = vec![];
        // ...and the middles of any indirect imports.
        let mut middles = vec![];
        loop {
            let chain = self.find_shortest_chain_with_excluded_modules_and_imports(
                &from_layer_with_descendants,
                &to_layer_with_descendants,
                &excluded_modules,
                &excluded_imports,
            )?;

            if chain.is_none() {
                break;
            }
            let chain = chain.unwrap();

            // Exclude this chain from further searching.
            for (importer, imported) in chain.iter().tuple_windows() {
                excluded_imports
                    .entry(*importer)
                    .or_default()
                    .insert(*imported);
            }

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
