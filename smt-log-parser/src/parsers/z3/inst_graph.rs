use fxhash::FxHashMap;
use gloo_console::log;
use petgraph::graph::NodeIndex;
use petgraph::Direction;
use petgraph::{
    stable_graph::{EdgeIndex, StableGraph},
    visit::{Dfs, EdgeRef},
    Direction::{Incoming, Outgoing},
};
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
    remove: bool,
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
    hidden_node: Option<NodeIndex>,
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
    pub inst_graph: StableGraph<NodeData, EdgeData>, // same as orig_inst_graph but possibly filtered
    orig_graph: StableGraph<NodeData, EdgeData>,
    node_of_line_nr: FxHashMap<usize, NodeIndex>, // line number => node-index
    indirect_edges_of_hidden_node: FxHashMap<NodeIndex, Vec<EdgeIndex>>, // stores for each hidden node which indirect edges were added
}

impl InstGraph {
    pub fn from(parser: &Z3Parser) -> Self {
        let mut inst_graph = Self::default();
        inst_graph.compute_instantiation_graph(parser);
        inst_graph
    }

    pub fn retain_nodes(&mut self, retain: impl Fn(&NodeData) -> bool) {
        self.inst_graph
            .retain_nodes(|graph, node_idx| retain(graph.node_weight(node_idx).unwrap()))
    }

    pub fn retain_nodes_and_reconnect(&mut self, retain: impl Fn(&NodeData) -> bool) {
        let nodes_to_remove: Vec<NodeIndex> = self
            .inst_graph
            .node_indices()
            .filter(|&node_idx| !retain(self.inst_graph.node_weight(node_idx).unwrap()))
            .collect();
        let mut edges_to_add: Vec<(NodeIndex, NodeIndex, NodeIndex)> = Vec::new();
        for &node in &nodes_to_remove {
            let preds: Vec<NodeIndex> =
                self.inst_graph.neighbors_directed(node, Incoming).collect();
            let succs: Vec<NodeIndex> =
                self.inst_graph.neighbors_directed(node, Outgoing).collect();
            self.inst_graph.remove_node(node);
            for &pred in &preds {
                for &succ in &succs {
                    if !nodes_to_remove.contains(&pred) && !nodes_to_remove.contains(&succ) {
                        edges_to_add.push((pred, node, succ));
                    }
                }
            }
        }
        for (pred, node, succ) in &edges_to_add {
            let indirect_edge = self.inst_graph.add_edge(
                *pred,
                *succ,
                EdgeData {
                    edge_type: EdgeType::Indirect,
                    hidden_node: Some(*node),
                },
            );
            if let Some(indirect_edges) = self.indirect_edges_of_hidden_node.get_mut(node) {
                indirect_edges.push(indirect_edge);
            } else {
                self.indirect_edges_of_hidden_node
                    .insert(*node, vec![indirect_edge]);
            }
        }
    }

    pub fn keep_n_most_costly(&mut self, n: usize) {
        // Only keep the max_instantiations most costly instantiations by sorting in
        // descending order of the cost.
        // In case two instantiations have the same cost, the instantiation with the lower
        // line number comes first in the order (is greater), or mathematically:
        // inst_b > inst_a iff (cost_b > cost_a or (cost_b = cost_a and line_nr_b < line_nr_a))
        // This is a total order since the line numbers are always guaranteed to be distinct
        // integers.
        let mut most_costly_insts: Vec<NodeIndex> = self.inst_graph.node_indices().collect();
        let cost_order = |node_a: &NodeIndex, node_b: &NodeIndex| {
            let node_a_data = self.inst_graph.node_weight(*node_a).unwrap();
            let node_b_data = self.inst_graph.node_weight(*node_b).unwrap();
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
        most_costly_insts.sort_unstable_by(cost_order);
        most_costly_insts.truncate(n);
        self.inst_graph
            .retain_nodes(|_, node| most_costly_insts.contains(&node));
    }

    pub fn remove_subtree_with_root(&mut self, root: NodeIndex) {
        let mut dfs = Dfs::new(&self.inst_graph, root);
        // iterate through all descendants of root and mark them to be removed
        while let Some(nx) = dfs.next(&self.inst_graph) {
            self.inst_graph[nx].remove = true;
        }
        // remove the marked nodes
        self.inst_graph
            .retain_nodes(|graph, node| !graph.node_weight(node).unwrap().remove)
    }

    pub fn only_show_ancestors(&mut self, node: NodeIndex) {
        // create new graph which is identical to original one except that all nodes have
        // remove = true instead of remove = false
        let mut ancestors = self.orig_graph.map(
            |_, &node| {
                let mut hidden_node = node;
                hidden_node.remove = true;
                hidden_node
            },
            |_, &edge| edge,
        );
        // visit all ancestors of node (argument) and set their remove-field to false since we want to retain them
        let orig_with_reversed_edges = petgraph::visit::Reversed(&self.orig_graph);
        let mut dfs = Dfs::new(orig_with_reversed_edges, node);
        while let Some(nx) = dfs.next(orig_with_reversed_edges) {
            ancestors[nx].remove = false;
        }
        // retain all ancestors of node, i.e., where remove-field was previously set to true
        ancestors.retain_nodes(|graph, node| !graph.node_weight(node).unwrap().remove);
        self.inst_graph = ancestors
    }

    pub fn reset(&mut self) {
        self.inst_graph = self.orig_graph.clone();
    }

    pub fn show_neighbours(&mut self, node: NodeIndex, direction: petgraph::Direction) {
        let neighbours: Vec<NodeIndex> = self
            .orig_graph
            .neighbors_directed(node, direction)
            .collect();
        let neighbours_edges: Vec<EdgeIndex> = neighbours
            .iter()
            .flat_map(|&neighbour| {
                self.orig_graph
                    .edges_directed(neighbour, Outgoing)
                    .chain(self.orig_graph.edges_directed(neighbour, Incoming))
                    .map(|e| e.id())
            })
            // .flatten()
            .collect();
        let mut new_inst_graph = self.orig_graph.filter_map(
            |node, &node_data| {
                if self.inst_graph.contains_node(node) || neighbours.contains(&node) {
                    Some(node_data)
                } else {
                    None
                }
            },
            |edge, &edge_data| {
                if self.inst_graph.edge_indices().any(|e| e == edge)
                    || neighbours_edges.contains(&edge)
                {
                    Some(edge_data)
                } else {
                    None
                }
            },
        );
        let redundant_indirect_edges: Vec<EdgeIndex> = neighbours
            .iter()
            .filter_map(|node| self.indirect_edges_of_hidden_node.remove(node))
            .flatten()
            .collect();
        let visible_indirect_edges = self
            .inst_graph
            .edge_indices()
            .filter(|&e| {
                self.inst_graph.edge_weight(e).unwrap().edge_type == EdgeType::Indirect
                    && !redundant_indirect_edges.contains(&e)
            })
            .map(|e| {
                let endpoints = self.inst_graph.edge_endpoints(e).unwrap();
                (
                    endpoints.0,
                    endpoints.1,
                    self.inst_graph.edge_weight(e).unwrap(),
                )
            });
        for (from, to, data) in visible_indirect_edges {
            let new_idx = new_inst_graph.add_edge(from, to, *data);
            let hidden_node = data.hidden_node.unwrap();
            let hidden_edges = self
                .indirect_edges_of_hidden_node
                .get_mut(&hidden_node)
                .unwrap();
            hidden_edges.push(new_idx);
        }
        self.inst_graph = new_inst_graph;
    }

    pub fn node_count(&self) -> usize {
        self.inst_graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.inst_graph.edge_count()
    }

    pub fn get_instantiation_info(&self, node_index: usize, parser: &Z3Parser) -> Option<InstInfo> {
        let NodeData { inst_idx, .. } = self
            .inst_graph
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

    // pub fn node_has_filtered_direct_neighbours(&self, node_idx: NodeIndex) -> bool {
    //     let nr_of_direct_neighbours = |graph: &StableGraph<NodeData, EdgeData>| {
    //         graph
    //             .edges_directed(node_idx, Incoming)
    //             .chain(graph.edges_directed(node_idx, Outgoing))
    //             .filter(|e| e.weight().edge_type == EdgeType::Direct)
    //             .count()
    //     };
    //     nr_of_direct_neighbours(&self.inst_graph) < nr_of_direct_neighbours(&self.orig_graph)
    // }

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
        let nr_of_direct_neighbours = |graph: &StableGraph<NodeData, EdgeData>| {
            graph
                .edges_directed(node_idx, direction)
                .filter(|e| e.weight().edge_type == EdgeType::Direct)
                .count()
        };
        nr_of_direct_neighbours(&self.inst_graph) < nr_of_direct_neighbours(&self.orig_graph)
    }

    fn compute_instantiation_graph(&mut self, parser: &Z3Parser) {
        for dep in &parser.dependencies {
            if let Some(to) = dep.to {
                let quant_idx = dep.quant;
                // let quant = parser.quantifiers.get(quant_idx).unwrap();
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
                    remove: false,
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
    }

    fn add_node(&mut self, node_data: NodeData) {
        let line_nr = node_data.line_nr;
        if !self.node_of_line_nr.contains_key(&line_nr) {
            let node = self.inst_graph.add_node(node_data);
            self.orig_graph.add_node(node_data);
            self.node_of_line_nr.insert(line_nr, node);
        }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (
            self.node_of_line_nr.get(&from),
            self.node_of_line_nr.get(&to),
        ) {
            self.inst_graph.add_edge(
                from_node_idx,
                to_node_idx,
                EdgeData {
                    edge_type: EdgeType::Direct,
                    hidden_node: None,
                },
            );
            self.orig_graph.add_edge(
                from_node_idx,
                to_node_idx,
                EdgeData {
                    edge_type: EdgeType::Direct,
                    hidden_node: None,
                },
            );
        }
    }
}
