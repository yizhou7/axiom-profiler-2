use std::collections::BTreeMap;

use petgraph::Graph;
use petgraph::graph::NodeIndex;

use super::z3parser::Z3Parser;
use crate::items::*;

impl Z3Parser {

    // /// Filter dependencies so that only the most constly instantiations are featured
    // fn filter_dependencies_by_cost(
    //     instantiations: &[Instantiation],
    //     dependencies: &[Dependency],
    // ) -> Vec<Dependency> {
    //     let inst_set = HashSet::<_>::from_iter(instantiations.iter().map(|inst| inst.line_no));
    //     dependencies
    //         .iter()
    //         .filter(|dep| inst_set.contains(&dep.to))
    //         .cloned()
    //         .collect::<Vec<Dependency>>()
    // }

    // /// Returns a vector of instantiations, sorted by cost
    // fn sort_instantiations_by_cost(
    //     instantiations: &BTreeMap<usize, Instantiation>,
    // ) -> Vec<Instantiation> {
    //     let mut inst_vec = instantiations
    //         .values()
    //         .cloned()
    //         .collect::<Vec<Instantiation>>();
    //     inst_vec.sort_unstable_by(|inst1, inst2| inst2.cost.partial_cmp(&inst1.cost).unwrap());
    //     inst_vec
    // }
    
    // /// Show only the `max_elements` most costly instantiations
    // fn filter_instantiations_by_cost(
    //     instantiations: &BTreeMap<usize, Instantiation>,
    //     max_elements: usize,
    // ) -> Vec<Instantiation> {
    //     Z3Parser::sort_instantiations_by_cost(instantiations)
    //         [..std::cmp::min(instantiations.len(), max_elements)]
    //         .to_vec()
    // }

    // fn get_sorted_dependencies(&self) -> Vec<Dependency> {
    //     let insts_sorted = Z3Parser::filter_instantiations_by_cost(&self.instantiations, 250);
    //     Self::filter_dependencies_by_cost(&insts_sorted, &self.dependencies)
    // }

    pub fn get_instantiation_graph(&self) -> (petgraph::Graph<usize, ()>, BTreeMap<usize, usize>) {
        let mut qi_graph = Graph::<usize, ()>::new();
        // let sorted_deps = self.get_sorted_dependencies();
        let is_theory_inst = |d: &Dependency| {
            d.quant_discovered
            // let quant_idx = d.quant;
            // if let Some(quant) = self.quantifiers.get(quant_idx) {
            //     match quant.kind {
            //         QuantKind::Other(_) => return true,
            //         _ => return false,
            //     }
            // }
            // false
        }; 
        // let is_theory_inst = |inst: usize| {
        //     if let Some(inst) = self.instantiations.get(InstIdx::from(inst)) {
        //         if let Some(quant) = self.quantifiers.get(inst.quant) {
        //             match quant.kind {
        //                 QuantKind::Other(_) => return true,
        //                 _ => return false,
        //             }
        //         }
        //     }
        //     false
        // };
        let fresh_line_nr = |qi_graph: &Graph::<usize, ()>, line_nr: usize| qi_graph.node_weights().all(|&line| line != line_nr);
        let mut node_of_line_nr: BTreeMap<usize, NodeIndex> = BTreeMap::new(); 
        let mut line_nr_of_node: BTreeMap<usize, usize> = BTreeMap::new(); 
        for dep in &self.dependencies {
            if !is_theory_inst(dep) { 
                if let (from, Some(to)) = (dep.from, dep.to) {
                    if fresh_line_nr(&qi_graph, to) {
                        let to_node = qi_graph.add_node(to);
                        node_of_line_nr.insert(to, to_node);
                        line_nr_of_node.insert(to_node.index(), to);
                    }
                    if from > 0 {
                        if fresh_line_nr(&qi_graph, from) {
                            let from_node = qi_graph.add_node(from);
                            node_of_line_nr.insert(from, from_node);
                            line_nr_of_node.insert(from_node.index(), from);
                        }
                        if let (Some(&from_node_idx), Some(&to_node_idx)) = (node_of_line_nr.get(&from), node_of_line_nr.get(&to)) {
                            qi_graph.add_edge(from_node_idx, to_node_idx, ());
                        }
                    }
                }
            }
        }
        (qi_graph, line_nr_of_node)
            // if is_not_theory_inst(&dep) {
                // check first that it has not yet been inserted into self.line_nr_of_node
                // if !node_of_line_nr.contains_key(&dep.to) {
                // if fresh_line_nr(&qi_graph, dep.to) {
                //     let to_node = qi_graph.add_node(dep.to);
                //     node_of_line_nr.insert(dep.to, to_node);
                //     line_nr_of_node.insert(to_node.index(), dep.to);
                // }
                // if dep.from != 0 {
                //     // check first that it has not yet been inserted into self.line_nr_of_node
                //     // if !self.node_of_line_nr.contains_key(&dep.from) {
                //     if fresh_line_nr(&qi_graph, dep.from) {
                //         let from_node = qi_graph.add_node(dep.from);
                //         node_of_line_nr.insert(dep.from, from_node);
                //         line_nr_of_node.insert(from_node.index(), dep.from);
                //     }
                //     if let Some(&from_node_idx) = node_of_line_nr.get(&dep.from) {
                //         if let Some(&to_node_idx) = node_of_line_nr.get(&dep.to) {
                //             qi_graph.add_edge(from_node_idx,to_node_idx, ());
                //         }
                //     }
                // }
            // }
    }
}