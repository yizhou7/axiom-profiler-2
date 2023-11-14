use fxhash::FxHashMap;
// use gloo_console::log;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use typed_index_collections::TiVec;
use std::fmt;

use crate::items::{InstIdx, Instantiation, TermIdx, QuantIdx, Quantifier};
use crate::parsers::LogParser;


use super::z3parser::Z3Parser;

#[derive(Clone, Copy)]
pub struct NodeData {
    pub line_nr: usize,
    is_theory_inst: bool,
    cost: f32,
    inst_idx: Option<InstIdx>,
    pub quant_idx: QuantIdx,
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line_nr)
    }
}

// impl fmt::Display for NodeData {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "label=\"{}\" style=filled, shape=oval, fillcolor=\"{}\", fontcolor=black ",
//         self.line_nr,
//         self.fill_colour)
//     }
// }

pub struct InstInfo {
    pub inst: Instantiation,
    pub formula: String,
    pub bound_terms: Vec<String>,
    pub yields_terms: Vec<String>,
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
        // First filter all nodes beyond max_line_nr.
        // If exclude_theory_inst is true, also filter all theory-solving instantiations.
        self.inst_graph = self.orig_inst_graph.filter_map(
            |_, &node| if node.line_nr <= max_line_nr && (!exclude_theory_inst || !node.is_theory_inst) {Some(node)} else {None}, 
            |_, &e| Some(e), // edges are retained unconditionally 
        );
        // Then only keep the max_instantiations most costly instantiations by sorting in
        // descending order of the cost. 
        // In case two instantiations have the same cost, the instantiation with the lower 
        // line number comes first in the order (is greater), or mathematically: 
        // This is a total order since the line numbers are always guaranteed to be distinct
        // integers.
        // inst_b > inst_a iff (cost_b > cost_a or (cost_b = cost_a and line_nr_b < line_nr_a))
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

    pub fn get_instantiation_info(&self, node_index: usize) -> Option<InstInfo> {
        let NodeData {inst_idx, ..} = self.inst_graph.node_weight(NodeIndex::new(node_index)).unwrap();
        if let Some(iidx) = inst_idx {
            let inst = self.parser.instantiations.get(*iidx).unwrap();
            let quant = self.parser.quantifiers.get(inst.quant).unwrap();
            let term_map = &self.parser.terms;
            let pretty_text_map = |tidxs: &Vec<TermIdx>| tidxs
                .iter()
                .map(|tidx| term_map.get(*tidx).unwrap())
                .map(|term| term.pretty_text(term_map))
                .collect::<Vec<String>>();
            let bound_terms = pretty_text_map(&inst.bound_terms);
            let yields_terms = pretty_text_map(&inst.yields_terms);
            let inst_info = InstInfo {
                inst: inst.clone(),
                formula: quant.pretty_text(term_map), 
                bound_terms, 
                yields_terms, 
            };
            Some(inst_info)
        } else {
            None
        }
    }

    pub fn get_quantifiers(&self) -> &TiVec<QuantIdx, Quantifier> {
        &self.parser.quantifiers
    }

    fn compute_instantiation_graph(&mut self, parser: &Z3Parser) {
        // use crate::items::QuantKind::*;
        // compute a colour map for the QuantKind::UnnamedQuant
        // let mut unnamed_quants = HashSet::new();
        // for quant in &parser.quantifiers {
        //     if let UnnamedQuant{id, ..} = &quant.kind {
        //         unnamed_quants.insert(id);
        //     }
        // }
        // let colours = make_hsv_colours(unnamed_quants.len());
        // let mut colour_map: HashMap<&usize, HSVColour> = HashMap::new();
        // for (idx, quant) in unnamed_quants.iter().enumerate() {
        //     colour_map.insert(quant, colours[idx]);
        // }
        // first add all nodes
        for dep in &parser.dependencies {
            if let Some(to) = dep.to {
                let quant_idx = dep.quant;
                let quant = parser.quantifiers.get(quant_idx).unwrap();
                let cost = quant.cost; 
                // let colour = if let UnnamedQuant{id, ..} = &quant.kind {
                //     *colour_map.get(id).unwrap()
                // } else {
                //     HSVColour::default()
                // };
                // log!("Quantifier at line nr ", to, " has kind ", quant.kind.to_string());
                self.add_node(NodeData{
                    line_nr: to, 
                    is_theory_inst: dep.quant_discovered, 
                    cost,
                    inst_idx: dep.to_iidx,
                    quant_idx, 
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