use petgraph::{graph::NodeIndex, visit::{Dfs, IntoNeighborsDirected, Reversed, Walker}, Direction};
use smt_log_parser::{display_with::{DisplayConfiguration, DisplayCtxt, DisplayWithCtxt}, items::{InstIdx, QuantIdx}, parsers::z3::graph::{raw::{Node, NodeKind, RawInstGraph}, InstGraph}, Z3Parser};

use super::svg_result::DEFAULT_NODE_COUNT;

pub const DEFAULT_FILTER_CHAIN: &[Filter] = &[
    Filter::IgnoreTheorySolving,
    Filter::MaxInsts(DEFAULT_NODE_COUNT),
];
pub const DEFAULT_DISABLER_CHAIN: &[(Disabler, bool)] = &[
    (Disabler::Smart, true),
    (Disabler::GivenEqualities, false),
    (Disabler::ENodes, false),
];

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Filter {
    MaxNodeIdx(usize),
    MinNodeIdx(usize),
    IgnoreTheorySolving,
    IgnoreQuantifier(Option<QuantIdx>),
    IgnoreAllButQuantifier(Option<QuantIdx>),
    MaxInsts(usize),
    MaxBranching(usize),
    ShowNeighbours(NodeIndex, Direction),
    VisitSourceTree(NodeIndex, bool),
    VisitSubTreeWithRoot(NodeIndex, bool),
    MaxDepth(usize),
    ShowLongestPath(NodeIndex),
    ShowNamedQuantifier(String),
    SelectNthMatchingLoop(usize),
    ShowMatchingLoopSubgraph,
}

impl Filter {
    pub fn apply(self, graph: &mut InstGraph, parser: &Z3Parser, config: DisplayConfiguration) -> FilterOutput {
        match self {
            Filter::MaxNodeIdx(max) => graph.raw.set_visibility_when(true, |idx: NodeIndex, _: &Node| idx.index() >= max),
            Filter::MinNodeIdx(min) => graph.raw.set_visibility_when(true, |idx: NodeIndex, _: &Node| idx.index() < min),
            Filter::IgnoreTheorySolving =>
                graph.raw.set_visibility_when(true, |_: NodeIndex, node: &Node| node.kind().inst().is_some_and(|i| parser[parser[i].match_].kind.is_discovered())),
            Filter::IgnoreQuantifier(qidx) =>
                graph.raw.set_visibility_when(true, |_: NodeIndex, node: &Node| node.kind().inst().is_some_and(|i| parser[parser[i].match_].kind.quant_idx() == qidx)),
            Filter::IgnoreAllButQuantifier(qidx) =>
            graph.raw.set_visibility_when(true, |_: NodeIndex, node: &Node| node.kind().inst().is_some_and(|i| parser[parser[i].match_].kind.quant_idx() != qidx)),
            Filter::MaxInsts(n) => graph.keep_first_n_cost(n),
            Filter::MaxBranching(n) => graph.keep_first_n_children(n),
            Filter::ShowNeighbours(nidx, direction) => {
                let nodes: Vec<_> = graph.raw.graph.neighbors_directed(nidx, direction).collect();
                graph.raw.set_visibility_many(false, nodes.into_iter())
            }
            Filter::VisitSubTreeWithRoot(nidx, retain) => {
                let nodes: Vec<_> = Dfs::new(&graph.raw.graph, nidx).iter(&graph.raw.graph).collect();
                graph.raw.set_visibility_many(!retain, nodes.into_iter())
            }
            Filter::VisitSourceTree(nidx, retain) => {
                let nodes: Vec<_> = Dfs::new(Reversed(&graph.raw.graph), nidx).iter(Reversed(&graph.raw.graph)).collect();
                graph.raw.set_visibility_many(!retain, nodes.into_iter())
            }
            Filter::MaxDepth(depth) =>
                graph.raw.set_visibility_when(true, |_: NodeIndex, node: &Node| node.fwd_depth.min as usize > depth),
            Filter::ShowLongestPath(nidx) =>
                return FilterOutput::LongestPath(graph.raw.show_longest_path_through(nidx)),
            Filter::ShowNamedQuantifier(name) => {
                let ctxt = DisplayCtxt { parser, config };
                graph.raw.set_visibility_when(false, |_: NodeIndex, node: &Node| node.kind().inst().is_some_and(|i|
                    parser[parser[i].match_].kind.quant_idx().map(|q| parser[q].kind.with(&ctxt).to_string()).is_some_and(|s| s == name)
                ))
            }
            // TODO: implement
            Filter::SelectNthMatchingLoop(n) => (),//return FilterOutput::MatchingLoopGeneralizedTerms(graph.show_nth_matching_loop(n, parser)),
            Filter::ShowMatchingLoopSubgraph => (),// graph.show_matching_loop_subgraph(),
        }
        FilterOutput::None
    }
    pub fn get_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub enum FilterOutput {
    LongestPath(Vec<NodeIndex>),
    MatchingLoopGeneralizedTerms(Vec<String>),
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disabler {
    ENodes,
    GivenEqualities,
    Smart,
}

impl Disabler {
    pub fn disable(self, idx: NodeIndex, graph: &RawInstGraph, _parser: &Z3Parser) -> bool {
        let node = &graph.graph[idx];
        match self {
            Disabler::ENodes => node.kind().enode().is_some(),
            Disabler::GivenEqualities => node.kind().eq_given().is_some(),
            Disabler::Smart => match node.kind() {
                NodeKind::ENode(_) => {
                    // Should only be 0 or 1
                    let parents = graph.graph.neighbors_directed(idx, Direction::Incoming).count();
                    let children = graph.graph.neighbors_directed(idx, Direction::Outgoing).count();
                    children == 0 || (parents == 1 && children == 1)
                },
                NodeKind::GivenEquality(_) => {
                    let parents = graph.graph.neighbors_directed(idx, Direction::Incoming).count();
                    let children = graph.graph.neighbors_directed(idx, Direction::Outgoing).count();
                    children == 0 || (parents == 1 && children == 1)
                },
                NodeKind::TransEquality(_) => {
                    let parents = graph.graph.neighbors_directed(idx, Direction::Incoming).count();
                    // Should be >= 1
                    let children = graph.graph.neighbors_directed(idx, Direction::Outgoing).count();
                    parents == 0 || (parents == 1 && children == 1)
                }
                NodeKind::Instantiation(_) => false,
            },
        }
    }
    pub fn apply(many: impl Iterator<Item = Disabler> + Clone, graph: &mut InstGraph, parser: &Z3Parser) {
        graph.reset_disabled_to(parser, |node, graph| many.clone().any(|d| d.disable(node, graph, parser)));
    }
}
