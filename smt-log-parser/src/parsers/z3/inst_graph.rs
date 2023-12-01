use fxhash::FxHashMap;
use gloo_console::log;
use petgraph::graph::{Edge, NodeIndex};
use petgraph::visit::IntoEdgeReferences;
use petgraph::{
    stable_graph::EdgeIndex,
    visit::{Dfs, EdgeRef},
    Direction::{Incoming, Outgoing},
};
use petgraph::{Direction, Graph};
use std::fmt;

use crate::items::{BlamedTermItem, InstIdx, QuantIdx, TermIdx};

use super::z3parser::Z3Parser;

#[derive(Clone, Copy, Default)]
pub struct NodeData {
    pub line_nr: usize,
    pub is_theory_inst: bool,
    cost: f32,
    pub inst_idx: Option<InstIdx>,
    pub quant_idx: QuantIdx,
    visible: bool,
    child_count: usize,
    parent_count: usize,
    pub orig_graph_idx: NodeIndex,
    cost_rank: usize,
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line_nr)
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum EdgeType {
    #[default]
    Direct,
    Indirect,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct EdgeData {
    pub edge_type: EdgeType,
}

impl fmt::Debug for EdgeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.edge_type {
            EdgeType::Direct => write!(f, "direct edge"),
            EdgeType::Indirect => write!(f, "indirect edge"),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct InstInfo {
    pub match_line_no: usize,
    pub line_no: Option<usize>,
    pub fingerprint: u64,
    pub resulting_term: Option<String>,
    pub z3_gen: Option<u32>,
    pub cost: f32,
    pub quant: QuantIdx,
    pub quant_discovered: bool,
    pub formula: String,
    pub pattern: Option<String>,
    pub yields_terms: Vec<String>,
    pub bound_terms: Vec<String>,
    pub blamed_terms: Vec<String>,
    pub equality_expls: Vec<String>,
    pub dep_instantiations: Vec<NodeIndex>,
    pub node_index: NodeIndex,
}

#[derive(Default, Clone)]
pub struct InstGraph {
    orig_graph: Graph<NodeData, EdgeData>,
    pub visible_graph: Graph<NodeData, EdgeData>,
    node_of_line_nr: FxHashMap<usize, NodeIndex>, // line number => node-index
    cost_ranked_node_indices: Vec<NodeIndex>,
}

impl InstGraph {
    pub fn from(parser: &Z3Parser) -> Self {
        let mut inst_graph = Self::default();
        inst_graph.compute_instantiation_graph(parser);
        inst_graph
    }

    pub fn retain_nodes(&mut self, retain: impl Fn(&NodeData) -> bool) {
        for node in self.orig_graph.node_weights_mut() {
            if !retain(node) {
                node.visible = false;
            }
        }
    }

    pub fn retain_visible_nodes_and_reconnect(&mut self) -> (usize, usize) {
        // retain all visible nodes
        let mut new_inst_graph = self.orig_graph.filter_map(
            |_, &node| {
                if node.visible {
                    Some(node)
                } else {
                    None
                }
            },
            |_, &edge_data| Some(edge_data),
        );
        // remember all direct edges (will be added to the graph in the end)
        let direct_edges = new_inst_graph
            .raw_edges()
            .iter()
            .cloned()
            .collect::<Vec<Edge<EdgeData>>>();
        // nodes with missing children
        let out_set: Vec<NodeIndex> = new_inst_graph
            .node_indices()
            .into_iter()
            .filter(|node| {
                let new_child_count = new_inst_graph.neighbors_directed(*node, Outgoing).count();
                let old_child_count = new_inst_graph.node_weight(*node).unwrap().child_count;
                new_child_count < old_child_count
            })
            .collect();
        // nodes with missing parents
        let in_set: Vec<NodeIndex> = new_inst_graph
            .node_indices()
            .into_iter()
            .filter(|node| {
                new_inst_graph.neighbors_directed(*node, Incoming).count()
                    < new_inst_graph.node_weight(*node).unwrap().parent_count
            })
            .collect();
        // remove all (direct) edges since we now want to compute the transitive reduction of the indirect edges
        new_inst_graph.clear_edges();
        // add all edges (u,v) in out_set x in_set to the new_inst_graph where v is reachable from u in the original graph
        // i.e., all indirect edges
        for &u in &out_set {
            for &v in &in_set {
                let old_u = new_inst_graph.node_weight(u).unwrap().orig_graph_idx;
                let old_v = new_inst_graph.node_weight(v).unwrap().orig_graph_idx;
                if old_u != old_v
                    && petgraph::algo::has_path_connecting(&self.orig_graph, old_u, old_v, None)
                {
                    new_inst_graph.add_edge(
                        u,
                        v,
                        EdgeData {
                            edge_type: EdgeType::Indirect,
                        },
                    );
                }
            }
        }
        // compute transitive reduction to minimize |E| and not clutter the graph
        let toposorted_dag = petgraph::algo::toposort(&new_inst_graph, None).unwrap();
        let (intermediate, _) = petgraph::algo::tred::dag_to_toposorted_adjacency_list::<_, u32>(
            &new_inst_graph,
            &toposorted_dag,
        );
        // in tred, the node indices are replaced by their topological rank
        // but revmap can be used to map from indices in new_inst_graph to indices in tred
        let (tred, _) = petgraph::algo::tred::dag_transitive_reduction_closure(&intermediate);
        // remove all edges since we only want the direct edges and the indirect edges in the transitive reduction in the final graph
        new_inst_graph.clear_edges();
        // add all direct edges to new_inst_graph that we removed previously
        for direct_edge in direct_edges {
            new_inst_graph.add_edge(
                direct_edge.source(),
                direct_edge.target(),
                direct_edge.weight,
            );
        }
        // add all indirect edges from transitive reduction
        for indirect_edge in tred.edge_references() {
            new_inst_graph.add_edge(
                toposorted_dag[indirect_edge.source() as usize],
                toposorted_dag[indirect_edge.target() as usize],
                EdgeData {
                    edge_type: EdgeType::Indirect,
                },
            );
        }
        self.visible_graph = new_inst_graph;
        (
            self.visible_graph.node_count(),
            self.visible_graph.edge_count(),
        )
    }

    pub fn keep_n_most_costly(&mut self, n: usize) {
        let visible_nodes: Vec<NodeIndex> = self
            .orig_graph
            .node_indices()
            .filter(|n| self.orig_graph.node_weight(*n).unwrap().visible)
            .collect();
        let nth_costliest_visible_node = self
            .cost_ranked_node_indices
            .iter()
            .filter(|nidx| visible_nodes.contains(nidx))
            .take(n)
            .last()
            .unwrap();
        let nth_largest_cost_rank = self
            .orig_graph
            .node_weight(*nth_costliest_visible_node)
            .unwrap()
            .cost_rank;
        // among the visible nodes keep those whose cost-rank
        // is larger than the cost rank of the n-th costliest
        self.retain_nodes(|node| node.visible && node.cost_rank <= nth_largest_cost_rank);
    }

    pub fn visit_descendants(&mut self, root: NodeIndex, retain: bool) {
        let mut dfs = Dfs::new(&self.orig_graph, root);
        while let Some(nx) = dfs.next(&self.orig_graph) {
            self.orig_graph[nx].visible = retain;
        }
    }

    pub fn visit_ancestors(&mut self, node: NodeIndex, retain: bool) {
        let mut dfs = Dfs::new(petgraph::visit::Reversed(&self.orig_graph), node);
        while let Some(nx) = dfs.next(petgraph::visit::Reversed(&self.orig_graph)) {
            self.orig_graph[nx].visible = retain;
        }
    }

    pub fn reset(&mut self) {
        for node in self.orig_graph.node_weights_mut() {
            node.visible = true;
        }
        self.visible_graph = self.orig_graph.clone();
    }

    pub fn show_neighbours(&mut self, node: NodeIndex, direction: petgraph::Direction) {
        let neighbour_indices: Vec<NodeIndex> = self
            .orig_graph
            .neighbors_directed(node, direction)
            .collect();
        for neighbour in neighbour_indices {
            self.orig_graph.node_weight_mut(neighbour).unwrap().visible = true;
        }
    }

    pub fn get_instantiation_info(&self, node_index: usize, parser: &Z3Parser) -> Option<InstInfo> {
        let NodeData { inst_idx, .. } = self
            .orig_graph
            .node_weight(NodeIndex::new(node_index))
            .unwrap();
        if let Some(iidx) = inst_idx {
            let inst = parser.instantiations.get(*iidx).unwrap();
            let quant = parser.quantifiers.get(inst.quant).unwrap();
            let term_map = &parser.terms;
            let prettify = |tidx: &TermIdx| {
                let term = parser.terms.get(*tidx).unwrap();
                term.pretty_text(term_map)
            };
            let prettify_all = |tidxs: &Vec<TermIdx>| {
                tidxs
                    .iter()
                    .map(|tidx| term_map.get(*tidx).unwrap())
                    .map(|term| term.pretty_text(term_map))
                    .collect::<Vec<String>>()
            };
            let pretty_blamed_terms = inst
                .blamed_terms
                .iter()
                .map(|term| match term {
                    BlamedTermItem::Single(t) => prettify(t),
                    BlamedTermItem::Pair(t1, t2) => format!("{} = {}", prettify(t1), prettify(t2)),
                })
                .collect::<Vec<String>>();
            let inst_info = InstInfo {
                match_line_no: inst.match_line_no,
                line_no: inst.line_no,
                fingerprint: *inst.fingerprint,
                resulting_term: if let Some(t) = inst.resulting_term {
                    Some(prettify(&t))
                } else {
                    None
                },
                z3_gen: inst.z3_gen,
                cost: inst.cost,
                quant: inst.quant,
                quant_discovered: inst.quant_discovered,
                formula: quant.pretty_text(term_map),
                pattern: if let Some(t) = inst.pattern {
                    Some(prettify(&t))
                } else {
                    None
                },
                yields_terms: prettify_all(&inst.yields_terms),
                bound_terms: prettify_all(&inst.bound_terms),
                blamed_terms: pretty_blamed_terms,
                equality_expls: prettify_all(&inst.equality_expls),
                dep_instantiations: Vec::new(),
                node_index: NodeIndex::new(node_index),
            };
            Some(inst_info)
        } else {
            None
        }
    }

    pub fn node_has_filtered_children(&self, node_idx: NodeIndex) -> bool {
        self.node_has_filtered_direct_neighbours(node_idx, Outgoing)
    }

    pub fn node_has_filtered_parents(&self, node_idx: NodeIndex) -> bool {
        self.node_has_filtered_direct_neighbours(node_idx, Incoming)
    }

    fn node_has_filtered_direct_neighbours(
        &self,
        node_idx: NodeIndex,
        direction: Direction,
    ) -> bool {
        let neighbours = self
            .orig_graph
            .edges_directed(node_idx, direction)
            .filter(|e| e.weight().edge_type == EdgeType::Direct)
            .map(|e| match direction {
                Outgoing => e.target(),
                Incoming => e.source(),
            });
        let (visible_neighbours, hidden_neighbours): (Vec<NodeIndex>, Vec<NodeIndex>) =
            neighbours.partition(|n| self.orig_graph.node_weight(*n).unwrap().visible);
        let nr_visible_neighbours = visible_neighbours.len();
        let nr_hidden_neighbours = hidden_neighbours.len();
        nr_visible_neighbours < nr_hidden_neighbours + nr_visible_neighbours
    }

    fn compute_instantiation_graph(&mut self, parser: &Z3Parser) {
        for dep in &parser.dependencies {
            if let Some(to) = dep.to {
                let quant_idx = dep.quant;
                let cost = parser
                    .instantiations
                    .get(dep.to_iidx.unwrap())
                    .unwrap()
                    .cost;
                self.add_node(NodeData {
                    line_nr: to,
                    is_theory_inst: dep.quant_discovered,
                    cost,
                    inst_idx: dep.to_iidx,
                    quant_idx,
                    visible: true,
                    child_count: 0,
                    parent_count: 0,
                    orig_graph_idx: NodeIndex::default(),
                    cost_rank: 0,
                });
            }
        }
        // then add all edges between nodes
        for dep in &parser.dependencies {
            let from = dep.from;
            if let Some(to) = dep.to {
                if from > 0 {
                    self.add_edge(from, to);
                }
            }
        }
        // precompute number of children and parents of each node
        self.orig_graph = self.orig_graph.map(
            |nidx, data| {
                let child_count = self.orig_graph.neighbors_directed(nidx, Outgoing).count();
                let parent_count = self.orig_graph.neighbors_directed(nidx, Incoming).count();
                let mut new_data = data.clone();
                new_data.child_count = child_count;
                new_data.parent_count = parent_count;
                new_data
            },
            |_, data| *data,
        );
        // precompute the cost-rank of all nodes by sorting the node_indices by our cost-order
        // in descending order and then assigning the rank to each node
        // Our cost-order is defined as follows:
        // inst_b > inst_a iff (cost_b > cost_a or (cost_b = cost_a and line_nr_b < line_nr_a))
        // This is a total order since the line numbers are always guaranteed to be distinct
        // integers.
        let mut cost_ranked_node_indices: Vec<NodeIndex> = self.orig_graph.node_indices().collect();
        let cost_order = |node_a: &NodeIndex, node_b: &NodeIndex| {
            let node_a_data = self.orig_graph.node_weight(*node_a).unwrap();
            let node_b_data = self.orig_graph.node_weight(*node_b).unwrap();
            if node_a_data.cost < node_b_data.cost {
                std::cmp::Ordering::Greater
            } else if node_a_data.cost == node_b_data.cost
                && node_b_data.line_nr < node_a_data.line_nr
            {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        };
        cost_ranked_node_indices.sort_unstable_by(cost_order);
        for (i, nidx) in cost_ranked_node_indices.iter().enumerate() {
            self.orig_graph.node_weight_mut(*nidx).unwrap().cost_rank = i;
        }
        self.cost_ranked_node_indices = cost_ranked_node_indices;
        self.visible_graph = self.orig_graph.clone();
    }

    fn add_node(&mut self, node_data: NodeData) {
        let line_nr = node_data.line_nr;
        if !self.node_of_line_nr.contains_key(&line_nr) {
            let node = self.orig_graph.add_node(node_data);
            self.node_of_line_nr.insert(line_nr, node);
            // store original node-index in each node
            // self.inst_graph.node_weight_mut(node).unwrap().orig_graph_idx = node;
            // store original node-idx such that when we compute reachability, we
            // can use the old indices.
            // this is necessary since filtering out nodes will changes node-indices
            // Also, using StableGraph where node-indices stay stable across removals
            // is not viable here since StableGraph does not implement NodeCompactIndexable
            // which is needed for petgraph::algo::tred::dag_to_toposorted_adjacency_list
            self.orig_graph
                .node_weight_mut(node)
                .unwrap()
                .orig_graph_idx = node;
        }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (
            self.node_of_line_nr.get(&from),
            self.node_of_line_nr.get(&to),
        ) {
            self.orig_graph.add_edge(
                from_node_idx,
                to_node_idx,
                EdgeData {
                    edge_type: EdgeType::Direct,
                },
            );
        }
    }
}
