use crate::errors::GrimpResult;
use crate::graph::pathfinding;
use crate::graph::{ExtendWithDescendants, Graph, ModuleToken};
use rustc_hash::{FxHashMap, FxHashSet};
use tap::Conv;

impl Graph {
    pub fn find_shortest_cycle(
        &self,
        module: ModuleToken,
        as_package: bool,
    ) -> GrimpResult<Option<Vec<ModuleToken>>> {
        let modules: Vec<ModuleToken> = if as_package {
            let mut vec = module.conv::<Vec<_>>().with_descendants(self);
            vec.sort_by_key(|token| self.get_module(*token).unwrap().name());
            vec
        } else {
            let vec: Vec<_> = module.conv::<Vec<ModuleToken>>();
            vec
        };
        pathfinding::find_shortest_cycle(
            self,
            &modules,
            &FxHashSet::default(),
            &FxHashMap::default(),
        )
    }
}
