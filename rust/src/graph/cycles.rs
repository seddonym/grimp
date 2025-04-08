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
        if as_package {
            pathfinding::find_shortest_cycle(
                self,
                &module.conv::<FxHashSet<_>>().with_descendants(self),
                &FxHashSet::default(),
                &FxHashMap::default(),
            )
        } else {
            pathfinding::find_shortest_cycle(
                self,
                &module.conv::<FxHashSet<_>>(),
                &FxHashSet::default(),
                &FxHashMap::default(),
            )
        }
    }
}
