use std::collections::BTreeMap;

use petgraph::Graph;
use petgraph::graph::NodeIndex;
use gloo_console::log;

use super::z3parser::Z3Parser;

#[derive(Default)]
pub struct InstGraph {
    pub inst_graph: Graph::<usize, ()>, // weights are the line numbers and have type usize
    pub line_nr_of_node: BTreeMap<usize, usize>, // node-index => line number
    node_of_line_nr: BTreeMap<usize, NodeIndex>, // line number => node-index
}

impl InstGraph {
    fn fresh_line_nr(&self, line_nr: usize) -> bool {
        self.inst_graph.node_weights().all(|&line| line != line_nr)
    } 

    pub fn add_node(&mut self, line_nr: usize) {
        if self.fresh_line_nr(line_nr) {
            let node = self.inst_graph.add_node(line_nr);
            self.node_of_line_nr.insert(line_nr, node);
            self.line_nr_of_node.insert(node.index(), line_nr);
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (self.node_of_line_nr.get(&from), self.node_of_line_nr.get(&to)) {
            self.inst_graph.add_edge(from_node_idx, to_node_idx, ());
        }
    }
}

impl Z3Parser {
    pub fn get_instantiation_graph(&self) -> InstGraph {
        let mut graph = InstGraph::default(); 
        let insts = &self.instantiations;
        // quant_discovered <=> instantiation not due to pattern-match in e-graph
        for to_inst in insts.iter().filter(|inst| !inst.quant_discovered) {
            if let Some(to) = to_inst.line_no {
                graph.add_node(to);
                let from_iidxs = &to_inst.dep_instantiations;
                for from_inst in from_iidxs.iter().filter_map(|&iidx| insts.get(iidx)) {
                    if let Some(from) = from_inst.line_no {
                        if from > 0 {
                            graph.add_node(from);
                            graph.add_edge(from, to);
                        }
                    }
                }
            }
        }
        graph
    }
}