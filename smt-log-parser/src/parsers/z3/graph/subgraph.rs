use fxhash::FxHashSet;
use petgraph::{graph::{DiGraph, EdgeIndex, Neighbors, NodeIndex}, Directed, Direction::{self, Incoming}, Undirected};
use roaring::RoaringBitmap;

pub struct Subgraph {
    pub(super) nodes: Vec<NodeIndex>,
    /// `reach_fwd[idx]` gives the set of nodes that can be reached from `idx`
    pub reach_fwd: TransitiveClosure,
    /// `reach_bwd[idx]` gives the set of nodes that can reach `idx`
    pub reach_bwd: TransitiveClosure,
}

pub struct VisitBox<D: VisitMap<NodeIndex>> {
    pub dfs: D,
}

impl Subgraph {
    pub fn new<N, E, D: VisitMap<NodeIndex>>(node: NodeIndex, graph: &mut DiGraph<N, E>, mut visit: VisitBox<D>, mut f: impl FnMut(&mut N, u32), c: impl Fn(&N) -> u32) -> (Self, VisitBox<D>) {
        let mut start_nodes = Vec::new();

        let mut un_graph = std::mem::replace(graph, DiGraph::new()).into_edge_type::<Undirected>();
        let mut dfs: Dfs<NodeIndex, _> = petgraph::visit::Dfs::from_parts(Vec::new(), visit.dfs);
        dfs.move_to(node);
        while let Some(node) = dfs.next(&un_graph) {
            let di_graph = un_graph.into_edge_type::<Directed>();
            let has_parents = di_graph.neighbors_directed(node, Incoming).next().is_some();
            un_graph = di_graph.into_edge_type();
            if !has_parents {
                start_nodes.push(node);
            }
        }
        visit.dfs = dfs.discovered;
        *graph = un_graph.into_edge_type();

        // OPTIMISATION: use a `VisitMap` from `VisitBox` to avoid allocating a
        // `FxHashSet` here (as well as the need for `SubgraphStartNodes`).
        let mut topo = petgraph::visit::Topo::new(&SubgraphStartNodes { start_nodes: &start_nodes, graph });
        let mut nodes = Vec::new();
        let mut count = 0_u32;
        while let Some(node) = topo.next(&SubgraphStartNodes { start_nodes: &start_nodes, graph }) {
            f(&mut graph[node], count as u32);
            count += 1;
            nodes.push(node);
        }

        // Transitive closure
        let mut reach_fwd = TransitiveClosure(vec![RoaringBitmap::new(); nodes.len()]);
        {
            let mut reach_fwd = &mut *reach_fwd.0;
            let mut reverse_topo = nodes.iter().enumerate().rev();
            while let (Some((idx, node)), Some((curr, others))) = (reverse_topo.next(), reach_fwd.split_last_mut()) {
                reach_fwd = others;
                curr.insert(idx as u32);
                for parent in graph.neighbors_directed(*node, Incoming) {
                    let parent = c(&graph[parent]);
                    reach_fwd[parent as usize] |= &*curr;
                }
            }
        }

        // let mut tc_combined = RoaringBitmap::new();
        // for (idx, tc) in transitive_closures.into_iter().enumerate() {
        //     let offset = (idx * nodes.len()) as u32;
        //     tc_combined.append(tc.into_iter().map(|x| x + offset));
        // }

        let mut reach_bwd = TransitiveClosure(vec![RoaringBitmap::new(); nodes.len()]);
        for (idx, reach_bwd) in reach_bwd.0.iter_mut().enumerate() {
            reach_bwd.append(reach_fwd.0.iter().enumerate().filter_map(
                |(i, tc)| if tc.contains(idx as u32) { Some(i as u32) } else { None }
            )).ok();
        }

        (Self { nodes, reach_fwd, reach_bwd }, visit)
    }

}

pub struct TransitiveClosure(Vec<RoaringBitmap>);
impl TransitiveClosure {
    pub fn in_transitive_closure(&self, from: u32, to: u32) -> bool {
        // self.reach_fwd.contains(from * self.nodes.len() as u32 + to)
        self.0[from as usize].contains(to)
    }
    pub fn reachable_from(&self, from: u32) -> impl Iterator<Item = u32> + '_ {
        // (0..self.nodes.len() as u32).filter(move |&to| self.in_transitive_closure(from, to))
        self.0[from as usize].iter()
    }
    pub fn reachable_from_many(&self, from: impl Iterator<Item = u32>) -> RoaringBitmap {
        let mut reachable = RoaringBitmap::new();
        for from in from {
            reachable |= &self.0[from as usize];
        }
        reachable
    }
}

// Graph wrapper for Topo walk

pub(super) struct SubgraphStartNodes<'g, N, E> {
    pub(super) start_nodes: &'g Vec<NodeIndex>,
    pub(super) graph: &'g DiGraph<N, E>,
}
use petgraph::visit::*;
impl<N, E> GraphBase for SubgraphStartNodes<'_, N, E> {
    type NodeId = NodeIndex;
    type EdgeId = EdgeIndex;
}
impl<'a, N, E> IntoNodeIdentifiers for &'a SubgraphStartNodes<'_, N, E> {
    type NodeIdentifiers = std::iter::Copied<std::slice::Iter<'a, NodeIndex>>;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.start_nodes.iter().copied()
    }
}
impl<'a, N, E> IntoNeighbors for &'a SubgraphStartNodes<'_, N, E> {
    type Neighbors = Neighbors<'a, E>;
    fn neighbors(self, n: Self::NodeId) -> Self::Neighbors {
        self.graph.neighbors(n)
    }
}
impl<'a, N, E> IntoNeighborsDirected for &'a SubgraphStartNodes<'_, N, E> {
    type NeighborsDirected = Neighbors<'a, E>;
    fn neighbors_directed(self, n: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        self.graph.neighbors_directed(n, d)
    }
}
impl<'a, N, E> Visitable for &'a SubgraphStartNodes<'_, N, E> {
    type Map = FxHashSet<NodeIndex>;
    fn visit_map(&self) -> Self::Map {
        FxHashSet::default()
    }
    fn reset_map(self: &Self, map: &mut Self::Map) {
        map.clear()
    }
}
