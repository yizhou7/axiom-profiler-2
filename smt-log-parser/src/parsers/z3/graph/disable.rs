use petgraph::graph::NodeIndex;

use crate::Z3Parser;

use super::{raw::{NodeState, RawInstGraph}, InstGraph};


impl InstGraph {
    /// Updates which nodes are disabled based on the predicate `f`. The
    /// `hidden` of all nodes is reset to `false` so any filters will need to be
    /// reapplied. The default analyses are also reapplied.
    pub fn reset_disabled_to(&mut self, parser: &Z3Parser, f: impl Fn(NodeIndex, &RawInstGraph) -> bool) {
        self.raw.reset_disabled_to_raw(f);
        self.initialise_default(parser);
    }
}

impl RawInstGraph {
    /// Updates which nodes are disabled based on the predicate `f`. The
    /// `hidden` of all non-disabled nodes is reset to `false` so any filters
    /// will need to be reapplied.
    /// 
    /// Analyses should be reapplied after this function is called!
    pub fn reset_disabled_to_raw(&mut self, f: impl Fn(NodeIndex, &RawInstGraph) -> bool) {
        for node in self.graph.node_indices() {
            if f(node, self) {
                self.stats.set_state(&mut self.graph[node], NodeState::Disabled);
            }
        }
    }
}
