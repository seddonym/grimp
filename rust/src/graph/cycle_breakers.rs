use crate::errors::GrimpResult;
use crate::graph::{Graph, ModuleToken};
use rustc_hash::FxHashSet;

impl Graph {
    pub fn nominate_cycle_breakers(
        &self,
        _package: ModuleToken,
    ) -> GrimpResult<FxHashSet<(ModuleToken, ModuleToken)>> {
        Ok(FxHashSet::default())
    }
}
