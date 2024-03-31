use std::{fmt, ops::{Index, IndexMut}};

use petgraph::{graph::{DiGraph, NodeIndex}, visit::Reversed, Direction::{self, Incoming, Outgoing}};
use typed_index_collections::TiVec;

use crate::{items::{ENodeIdx, EqGivenIdx, EqTransIdx, EqualityExpl, GraphIdx, InstIdx, TransitiveExplSegment}, Z3Parser};

use super::subgraph::Subgraph;

pub struct RawInstGraph {
    pub graph: DiGraph<Node, EdgeKind>,
    enode_idx: NodeIndex,
    eq_given_idx: NodeIndex,
    eq_trans_idx: NodeIndex,
    inst_idx: NodeIndex,

    pub(super) stats: GraphStats,
}

impl RawInstGraph {
    pub fn new(parser: &Z3Parser) -> Self {
        let total_nodes = parser.insts.insts.len()
            + parser.egraph.enodes.len()
            + parser.egraph.equalities.given.len()
            + parser.egraph.equalities.transitive.len();
        let edges_lower_bound = parser.insts.insts.len()
            + parser.egraph.equalities.transitive.len();
        let mut graph = DiGraph::with_capacity(total_nodes, edges_lower_bound);
        let enode_idx = NodeIndex::new(graph.node_count());
        for enode in parser.egraph.enodes.keys() {
            graph.add_node(Node::new(NodeKind::ENode(enode)));
        }
        let eq_given_idx = NodeIndex::new(graph.node_count());
        for eq_given in parser.egraph.equalities.given.keys() {
            graph.add_node(Node::new(NodeKind::GivenEquality(eq_given)));
        }
        let eq_trans_idx = NodeIndex::new(graph.node_count());
        for eq_trans in parser.egraph.equalities.transitive.keys() {
            graph.add_node(Node::new(NodeKind::TransEquality(eq_trans)));
        }
        let inst_idx = NodeIndex::new(graph.node_count());
        for inst in parser.insts.insts.keys() {
            graph.add_node(Node::new(NodeKind::Instantiation(inst)));
        }
        let stats = GraphStats { hidden: graph.node_count() as u32, disabled: 0, generation: 0 };
        let mut self_ = RawInstGraph { graph, enode_idx, eq_given_idx, eq_trans_idx, inst_idx, stats };

        // Add instantiation blamed and yield edges
        for (idx, inst) in parser.insts.insts.iter_enumerated() {
            for yields in inst.yields_terms.iter() {
                self_.add_edge(idx, *yields, EdgeKind::Yield);
            }
            for (i, blame) in parser.insts.matches[inst.match_].trigger_matches().enumerate() {
                let trigger_term = i as u16;
                self_.add_edge(blame.enode(), idx, EdgeKind::Blame { trigger_term });
                for (i, eq) in blame.equalities().enumerate() {
                    self_.add_edge(eq, idx, EdgeKind::BlameEq { trigger_term, eq_order: i as u16 });
                }
            }
        }

        // Add given equality created edges
        for (idx, eq) in parser.egraph.equalities.given.iter_enumerated() {
            match eq {
                EqualityExpl::Root { .. } => (),
                EqualityExpl::Literal { eq, .. } =>
                    self_.add_edge(*eq, idx, EdgeKind::EqualityFact),
                EqualityExpl::Congruence { arg_eqs, .. } => for arg_eq in arg_eqs.iter() {
                    self_.add_edge(*arg_eq, idx, EdgeKind::EqualityCongruence);
                },
                EqualityExpl::Theory { .. } => (),
                EqualityExpl::Axiom { .. } => (),
                EqualityExpl::Unknown { .. } => (),
            }
        }

        // Add transitive equality created edges
        for (idx, eq) in parser.egraph.equalities.transitive.iter_enumerated() {
            let mut all = eq.all(true);
            while let Some(parent) = all.next() {
                match parent {
                    TransitiveExplSegment::Leaf(eq) =>
                        self_.add_edge(*eq, idx, EdgeKind::TEqualitySimple),
                    TransitiveExplSegment::TransitiveFwd(eq) =>
                        self_.add_edge(*eq, idx, EdgeKind::TEqualityTransitive),
                    TransitiveExplSegment::TransitiveBwd(eq) =>
                        self_.add_edge(*eq, idx, EdgeKind::TEqualityTransitiveBwd),
                }
            }
        }

        self_
    }
    fn add_edge(&mut self, source: impl IndexesInstGraph, target: impl IndexesInstGraph, kind: EdgeKind) {
        self.graph.add_edge(source.index(self), target.index(self), kind);
    }

    pub fn partition(&mut self) -> TiVec<GraphIdx, Subgraph> {
        let mut subgraphs = TiVec::new();
        for node in self.graph.node_indices() {
            let has_parents = self.graph.neighbors_directed(node, Incoming).next().is_some();
            if has_parents {
                continue;
            }
            let has_children = self.graph.neighbors_directed(node, Outgoing).next().is_some();
            if !has_children {
                continue;
            }
            if self.graph[node].subgraph.is_some() {
                continue;
            }

            // Construct subgraph
            let idx = subgraphs.next_key();
            let subgraph = Subgraph::new(
                node,
                &mut self.graph,
                |node, i| node.subgraph = Some((idx, i)),
                |node| node.subgraph.unwrap().1,
            );
            subgraphs.push(subgraph);
        }
        subgraphs
    }

    pub fn index(&self, kind: NodeKind) -> NodeIndex {
        match kind {
            NodeKind::ENode(enode) => enode.index(self),
            NodeKind::GivenEquality(eq) => eq.index(self),
            NodeKind::TransEquality(eq) => eq.index(self),
            NodeKind::Instantiation(inst) => inst.index(self),
        }
    }

    pub fn rev(&self) -> Reversed<&DiGraph<Node, EdgeKind>> {
        Reversed(&self.graph)
    }
    pub fn neighbors_directed(&self, node: NodeIndex, dir: Direction) -> Vec<NodeIndex> {
        let (mut disabled, mut enabled): (Vec<_>, Vec<_>) = self.graph.neighbors_directed(node, dir).partition(|n| self.graph[*n].disabled());
        while let Some(next) = disabled.pop() {
            for n in self.graph.neighbors_directed(next, dir) {
                if self.graph[n].disabled() {
                    disabled.push(n);
                } else {
                    enabled.push(n);
                }
            }
        }
        enabled
    }
}

pub struct GraphStats {
    pub hidden: u32,
    pub disabled: u32,
    /// How many times has the visibility of nodes been changed?
    /// Used to keep track of if the hidden graph needs to be recalculated.
    pub generation: u32,
}

impl GraphStats {
    pub fn set_state(&mut self, node: &mut Node, state: NodeState) -> bool {
        if node.state == state {
            return false;
        }
        self.generation = self.generation.wrapping_add(1);
        match node.state {
            NodeState::Disabled => self.disabled -= 1,
            NodeState::Hidden => self.hidden -= 1,
            _ => (),
        }
        match state {
            NodeState::Disabled => self.disabled += 1,
            NodeState::Hidden => self.hidden += 1,
            _ => (),
        }
        node.state = state;
        true
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    state: NodeState,
    pub cost: f64,
    pub fwd_depth: Depth,
    pub bwd_depth: Depth,
    pub subgraph: Option<(GraphIdx, u32)>,
    kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Disabled,
    Hidden,
    Visible,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Depth {
    /// What is the shortest path to a root/leaf
    pub min: u32,
    /// What is the longest path to a root/leaf
    pub max: u32,
}

impl Node {
    fn new(kind: NodeKind) -> Self {
        Self { state: NodeState::Hidden, cost: 0.0, fwd_depth: Depth::default(), bwd_depth: Depth::default(), subgraph: None, kind }
    }
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn disabled(&self) -> bool {
        matches!(self.state, NodeState::Disabled)
    }
    pub fn hidden(&self) -> bool {
        matches!(self.state, NodeState::Hidden)
    }
    pub fn visible(&self) -> bool {
        matches!(self.state, NodeState::Visible)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NodeKind {
    /// Corresponds to `ENodeIdx`.
    /// 
    /// **Parents:** will always have 0 or 1 parents, if 1 then this will be an `Instantiation`.\
    /// **Children:** arbitrary count, will always be either `Instantiation` or
    /// `GivenEquality` of type `EqualityExpl::Literal`.
    ENode(ENodeIdx),
    /// Corresponds to `EqGivenIdx`.
    /// 
    /// **Parents:** will always have 0 or 1 parents, if 1 then this will be an
    /// `ENode` or a `TransEquality` depending on if it's a `Literal` or
    /// `Congruence` resp.\
    /// **Children:** arbitrary count, will always be `TransEquality` of type.
    GivenEquality(EqGivenIdx),
    /// Corresponds to `EqTransIdx`.
    /// 
    /// **Parents:** arbitrary count, will always be `GivenEquality` or
    /// `TransEquality`. The number of immediately reachable `GivenEquality` con
    /// be found in `TransitiveExpl::given_len`.\
    /// **Children:** arbitrary count, can be `GivenEquality`, `TransEquality`
    /// or `Instantiation`.
    TransEquality(EqTransIdx),
    /// Corresponds to `InstIdx`.
    /// 
    /// **Parents:** arbitrary count, will always be `ENode` or `TransEquality`.\
    /// **Children:** arbitrary count, will always be `ENode`.
    Instantiation(InstIdx),
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::ENode(enode) => write!(f, "{enode:?}"),
            NodeKind::GivenEquality(eq) => write!(f, "{eq:?}"),
            NodeKind::TransEquality(eq) => write!(f, "{eq:?}"),
            NodeKind::Instantiation(inst) => write!(f, "{inst:?}"),
        }
    }
}

impl NodeKind {
    pub fn enode(&self) -> Option<ENodeIdx> {
        match self {
            Self::ENode(enode) => Some(*enode),
            _ => None,
        }
    }
    pub fn eq_given(&self) -> Option<EqGivenIdx> {
        match self {
            Self::GivenEquality(eq) => Some(*eq),
            _ => None,
        }
    }
    pub fn eq_trans(&self) -> Option<EqTransIdx> {
        match self {
            Self::TransEquality(eq) => Some(*eq),
            _ => None,
        }
    }
    pub fn inst(&self) -> Option<InstIdx> {
        match self {
            Self::Instantiation(inst) => Some(*inst),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    /// Instantiation -> ENode
    Yield,
    /// ENode -> Instantiation
    Blame { trigger_term: u16 },
    /// TransEquality -> Instantiation
    BlameEq { trigger_term: u16, eq_order: u16 },
    /// ENode -> GivenEquality (`EqualityExpl::Literal`)
    EqualityFact,
    /// TransEquality -> GivenEquality (`EqualityExpl::Congruence`)
    EqualityCongruence,
    /// GivenEquality -> TransEquality (`TransitiveExplSegment::Leaf`)
    TEqualitySimple,
    /// TransEquality -> TransEquality (`TransitiveExplSegment::TransitiveFwd`)
    TEqualityTransitive,
    /// TransEquality -bwd-> TransEquality (`TransitiveExplSegment::TransitiveBwd`)
    TEqualityTransitiveBwd,
}

pub(super) trait IndexesInstGraph {
    fn index(&self, graph: &RawInstGraph) -> NodeIndex;
}
impl IndexesInstGraph for ENodeIdx {
    fn index(&self, graph: &RawInstGraph) -> NodeIndex {
        NodeIndex::new(graph.enode_idx.index() + usize::from(*self))
    }
}
impl IndexesInstGraph for EqGivenIdx {
    fn index(&self, graph: &RawInstGraph) -> NodeIndex {
        NodeIndex::new(graph.eq_given_idx.index() + usize::from(*self))
    }
}
impl IndexesInstGraph for EqTransIdx {
    fn index(&self, graph: &RawInstGraph) -> NodeIndex {
        NodeIndex::new(graph.eq_trans_idx.index() + usize::from(*self))
    }
}
impl IndexesInstGraph for InstIdx {
    fn index(&self, graph: &RawInstGraph) -> NodeIndex {
        NodeIndex::new(graph.inst_idx.index() + usize::from(*self))
    }
}
impl IndexesInstGraph for NodeIndex {
    fn index(&self, _graph: &RawInstGraph) -> NodeIndex {
        *self
    }
}

impl<T: IndexesInstGraph> Index<T> for RawInstGraph {
    type Output = Node;
    fn index(&self, index: T) -> &Self::Output {
        &self.graph[index.index(self)]
    }
}
impl<T: IndexesInstGraph> IndexMut<T> for RawInstGraph {
    fn index_mut(&mut self, index: T) -> &mut Self::Output {
        let index = index.index(self);
        &mut self.graph[index]
    }
}
