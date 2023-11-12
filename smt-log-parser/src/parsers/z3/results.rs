use fxhash::FxHashMap;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use std::fmt;

use crate::parsers::LogParser;

use super::z3parser::Z3Parser;

#[derive(Clone, Copy)]
pub struct NodeData {
    line_nr: usize,
    is_theory_inst: bool,
    cost: f32,
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line_nr)
    }
}

#[derive(Default)]
pub struct InstGraph {
    orig_inst_graph: Graph::<NodeData, ()>, // weights are the line numbers and have type usize
    inst_graph: Graph::<NodeData, ()>, // same as orig_inst_graph but possibly filtered 
    node_of_line_nr: FxHashMap<usize, NodeIndex>, // line number => node-index
    parser: Z3Parser, // contains all information needed to construct this instantiation graph
}

impl InstGraph {

    pub fn from(trace_file_text: &str) -> Self {
        let mut inst_graph = Self::default();
        let parser = Z3Parser::from_str(trace_file_text).process_all();
        inst_graph.compute_instantiation_graph(&parser);
        inst_graph.parser = parser;
        inst_graph
    }

    pub fn filter(&mut self, settings: FilterSettings) -> &Graph::<NodeData, ()> {
        let FilterSettings{max_line_nr, exclude_theory_inst, max_instantiations} = settings;
        // first filter all nodes beyond max_line_nr
        self.inst_graph = self.orig_inst_graph.filter_map(
            |_, &node| if node.line_nr <= max_line_nr && (!exclude_theory_inst || !node.is_theory_inst) { Some(node) } else { None }, 
            |_, _| Some(()), 
        );
        // then only keep the max_instantiations most costly instantiations
        let mut most_costly_insts: Vec<NodeIndex> = self.inst_graph.node_indices().collect();
        most_costly_insts.sort_by(|node_a, node_b| {
            let node_a_data = self.inst_graph.node_weight(*node_a).unwrap();
            let node_b_data = self.inst_graph.node_weight(*node_b).unwrap();
            if node_a_data.cost < node_b_data.cost {
                return std::cmp::Ordering::Greater
            } else if node_a_data.cost == node_b_data.cost && node_b_data.line_nr < node_a_data.line_nr {
                return std::cmp::Ordering::Greater
            } else {
                return std::cmp::Ordering::Less
            }
        });
        most_costly_insts.truncate(max_instantiations);
        self.inst_graph.retain_nodes(|_, node| most_costly_insts.contains(&node));
        &self.inst_graph
    }

    fn compute_instantiation_graph(&mut self, parser: &Z3Parser) {
        // first add all nodes
        for dep in &parser.dependencies {
            if let Some(to) = dep.to {
                let qidx = dep.quant;
                let cost = parser.quantifiers.get(qidx).unwrap().cost; 
                self.add_node(NodeData{
                    line_nr: to, 
                    is_theory_inst: dep.quant_discovered, 
                    cost
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

    fn fresh_line_nr(&self, line_nr: usize) -> bool {
        // self.orig_inst_graph.node_weights().all(|&line| line != line_nr)
        self.orig_inst_graph.node_weights().all(|node| node.line_nr != line_nr)
    } 

    fn add_node(&mut self, node_data: NodeData) {
        let line_nr = node_data.line_nr;
        if self.fresh_line_nr(line_nr) {
            let node = self.orig_inst_graph.add_node(node_data);
            self.node_of_line_nr.insert(line_nr, node);
        }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (self.node_of_line_nr.get(&from), self.node_of_line_nr.get(&to)) {
            self.orig_inst_graph.add_edge(from_node_idx, to_node_idx, ());
        }
    }

    // fn retain_nodes_and_reconnect(&mut self, retain_if: impl Fn(&NodeData) -> bool) {
    //     let nodes_to_remove: Vec<NodeIndex> = self.inst_graph
    //         .node_indices()
    //         .filter(|&node_idx| !retain_if(self.inst_graph.node_weight(node_idx).unwrap()))
    //         .collect();
    //     // in first pass, just add all edges between predecessors and successors of nodes to be removed
    //     for node in nodes_to_remove {
    //         let preds: Vec<NodeIndex> = self.inst_graph.neighbors_directed(node, Incoming).collect();
    //         let succs: Vec<NodeIndex> = self.inst_graph.neighbors_directed(node, Outgoing).collect();
    //         for &pred in &preds {
    //             for &succ in &succs {
    //                 self.inst_graph.add_edge(pred, succ, ());
    //             }
    //         }
    //     }
    //     // in second pass, remove all nodes to be removed
    //     // self.inst_graph.retain_nodes(visit)
    // }
}

#[derive(PartialEq, Clone, Copy)]
pub struct FilterSettings {
    pub max_line_nr: usize,
    pub exclude_theory_inst: bool,
    pub max_instantiations: usize,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            max_line_nr: usize::MAX,
            exclude_theory_inst: true,
            max_instantiations: 250,
        }
    }
}