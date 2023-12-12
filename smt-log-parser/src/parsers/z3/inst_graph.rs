use fxhash::FxHashMap;
use gloo_console::log;
use petgraph::graph::{Edge, NodeIndex};
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{IntoEdgeReferences, Bfs, Topo};
use petgraph::{
    stable_graph::EdgeIndex,
    visit::{Dfs, EdgeRef},
    Direction::{Incoming, Outgoing},
};
use petgraph::{Direction, Graph};
use std::fmt;

use crate::items::{BlamedTermItem, InstIdx, QuantIdx, TermIdx, DepType, Dependency};

use super::z3parser::{Z3Parser, Prettify};

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
    branching_rank: usize,
    pub min_depth: Option<usize>,
    max_depth: usize, 
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line_nr)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum EdgeType {
    Direct(DepType),
    Indirect,
}

#[derive(Clone, Copy, PartialEq)]
pub struct EdgeData {
    pub edge_type: EdgeType,
    pub orig_graph_idx: Option<EdgeIndex>,
    pub blame_term_idx: Option<TermIdx>,
}

#[derive(PartialEq, Clone)]
pub struct EdgeInfo {
    pub edge_data: EdgeData,
    pub blame_term: String,
    pub from: NodeIndex,
    pub to: NodeIndex,
}

impl fmt::Debug for EdgeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.edge_type {
            EdgeType::Direct(_) => write!(f, "direct edge"),
            EdgeType::Indirect => write!(f, "indirect edge"),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
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
    branching_ranked_node_indices: Vec<NodeIndex>,
}

enum InstOrder {
    Branching,
    Cost
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

    pub fn retain_visible_nodes_and_reconnect(&mut self) -> (usize, usize, bool, bool) {
        let prev_node_count = self.visible_graph.node_count();
        let prev_edge_count = self.visible_graph.edge_count();
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
        // add all edges (u,v) in out_set x in_set to the new_inst_graph where v is reachable from u in the original graph
        // and (u,v) is not an edge in the original graph, i.e., all indirect edges
        for &u in &out_set {
            for &v in &in_set {
                let old_u = new_inst_graph.node_weight(u).unwrap().orig_graph_idx;
                let old_v = new_inst_graph.node_weight(v).unwrap().orig_graph_idx;
                if old_u != old_v && !self.orig_graph.contains_edge(old_u, old_v)
                    && petgraph::algo::has_path_connecting(&self.orig_graph, old_u, old_v, None)
                {
                    new_inst_graph.add_edge(
                        u,
                        v,
                        EdgeData {
                            edge_type: EdgeType::Indirect,
                            orig_graph_idx: None,
                            blame_term_idx: None,
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
        // add all indirect edges from transitive reduction that are not direct edges
        for indirect_edge in tred.edge_references() {
            // in tred, the node indices are replaced by their topological rank
            // hence we need to look up the index before adding to new_inst_graph
            let source = toposorted_dag[indirect_edge.source() as usize];
            let target = toposorted_dag[indirect_edge.target() as usize];
            // we only want indirect edges
            if !new_inst_graph.contains_edge(source, target) {
                new_inst_graph.add_edge(
                    source,
                    target,
                    EdgeData {
                        edge_type: EdgeType::Indirect,
                        orig_graph_idx: None,
                        blame_term_idx: None,
                    },
                );
            }
        }
        self.visible_graph = new_inst_graph;
        let curr_node_count = self.visible_graph.node_count();
        let curr_edge_count = self.visible_graph.edge_count();
        (
            self.visible_graph.node_count(),
            self.visible_graph.edge_count(),
            prev_node_count > curr_node_count,
            prev_edge_count > curr_edge_count
        )
    }

    pub fn keep_n_most_costly(&mut self, n: usize) {
        self.keep_n_highest_ranked(n, InstOrder::Cost)
    } 

    pub fn keep_n_most_branching(&mut self, n: usize) {
        self.keep_n_highest_ranked(n, InstOrder::Branching)
    } 

    fn keep_n_highest_ranked(&mut self, n: usize, order: InstOrder) {
        let ranked_node_indices = match order {
            InstOrder::Branching => &self.branching_ranked_node_indices,
            InstOrder::Cost => &self.cost_ranked_node_indices,
        };
        let visible_nodes: Vec<NodeIndex> = self
            .orig_graph
            .node_indices()
            .filter(|n| self.orig_graph.node_weight(*n).unwrap().visible)
            .collect();
        if let Some(nth_highest_ranked_visible_node) = ranked_node_indices  
            .iter()
            .filter(|nidx| visible_nodes.contains(nidx))
            .take(n)
            .last() {
            let nth_largest_rank = self
                .orig_graph
                .node_weight(*nth_highest_ranked_visible_node)
                .unwrap()
                .clone();
            // among the visible nodes keep those whose cost-rank
            // is larger than the cost rank of the n-th costliest
            match order {
                InstOrder::Branching => self.retain_nodes(|node| node.visible && node.branching_rank <= nth_largest_rank.branching_rank),
                InstOrder::Cost => self.retain_nodes(|node| node.visible && node.cost_rank <= nth_largest_rank.cost_rank),
            }
        }
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

    pub fn show_longest_path_through(&mut self, node: NodeIndex) -> Vec<NodeIndex> {
        // construct subtree rooted at selected node
        let mut subtree_rooted_at_node: StableGraph<NodeData, EdgeData> = StableGraph::from(self.orig_graph.clone());
        for node in subtree_rooted_at_node.node_weights_mut() {
            node.visible = false;
        }
        let mut dfs = Dfs::new(&subtree_rooted_at_node, node);
        while let Some(nx) = dfs.next(&subtree_rooted_at_node) {
            subtree_rooted_at_node[nx].visible = true;
        }
        subtree_rooted_at_node = subtree_rooted_at_node.filter_map(
            |_, node_data| if node_data.visible { Some(*node_data) } else { None },
            |_, edge| Some(*edge)
        );
        // traverse this subtree in topological order to compute longest distances from node
        let mut topo = Topo::new(&subtree_rooted_at_node);
        while let Some(nx) = topo.next(&subtree_rooted_at_node) {
            let parents = subtree_rooted_at_node.neighbors_directed(nx, Incoming); 
            let max_parent_depth = parents
                .map(|nx| subtree_rooted_at_node.node_weight(nx).unwrap().max_depth)
                .max(); 
            if let Some(_) = max_parent_depth {
                subtree_rooted_at_node[nx].max_depth += 1;
            }
        }
        let furthest_away_node_idx = subtree_rooted_at_node
            .node_weights()
            .max_by(|node_a, node_b| node_a.max_depth.cmp(&node_b.max_depth))
            .unwrap()
            .orig_graph_idx;
        // backtrack a longest path from furthest away node in subgraph until we reach the root
        // with respect to the subgraph, i.e., node
        // self.backtrack(Some(&subtree_rooted_at_node), furthest_away_node_idx);
        let mut longest_path: Vec<NodeIndex> = Vec::new();
        let mut visitor: Vec<NodeIndex>= Vec::new();
        if furthest_away_node_idx != node {
            visitor.push(furthest_away_node_idx);
        }
        while let Some(curr) = visitor.pop() {
            log!(format!("Visiting node {} on longest path", curr.index()));
            longest_path.push(curr);
            self.orig_graph[curr].visible = true;
            let curr_distance = subtree_rooted_at_node.node_weight(curr).unwrap().max_depth;
            // log!(format!("Node {} has distance {} from {}", curr.index(), curr_distance, ))
            let pred = subtree_rooted_at_node
                .neighbors_directed(curr, Incoming)
                .filter(|pred| { 
                    let pred_distance = subtree_rooted_at_node.node_weight(*pred).unwrap().max_depth; 
                    pred_distance == curr_distance - 1 
                })
                .last();
            if let Some(node) = pred {
                visitor.push(node);
            }
        } 
        // backtrack a longest path from node until we reach a root with respect to the original graph 
        visitor.push(node);
        while let Some(curr) = visitor.pop() {
            longest_path.push(curr);
            self.orig_graph[curr].visible = true;
            let curr_distance = self.orig_graph.node_weight(curr).unwrap().max_depth;
            // log!(format!("Node {} has distance {} from {}", curr.index(), curr_distance, ))
            let pred = self.orig_graph 
                .neighbors_directed(curr, Incoming)
                .filter(|pred| { 
                    let pred_distance = self.orig_graph.node_weight(*pred).unwrap().max_depth; 
                    pred_distance == curr_distance - 1 
                })
                .last();
            if let Some(node) = pred {
                visitor.push(node);
            }
        }
        longest_path
            .iter()
            .cloned()
            .rev()
            .collect::<Vec<NodeIndex>>()
    }
    
    // fn backtrack<T>(&mut self, graph: Option<T>, node: NodeIndex) where 
    // T: NodeIndexable<EdgeId = EdgeIndex, NodeId = NodeIndex> + DataMap<NodeWeight = NodeData> + IntoNeighborsDirected {
    //     let mut visitor: Vec<NodeIndex> = Vec::new();
    //     visitor.push(node);
    //     while let Some(curr) = visitor.pop() {
    //         self.orig_graph[curr].visible = true;
    //         let curr_distance = graph.unwrap_or(self.orig_graph).node_weight(curr).unwrap().max_depth;
    //         // log!(format!("Node {} has distance {} from {}", curr.index(), curr_distance, ))
    //         let pred = graph 
    //             .unwrap_or_default(self.orig_graph)
    //             .neighbors_directed(curr, Incoming)
    //             .filter(|pred| { 
    //                 let pred_distance = graph.unwrap_or(self.orig_graph).node_weight(*pred).unwrap().max_depth; 
    //                 pred_distance == curr_distance - 1 
    //             })
    //             .last();
    //         if let Some(node) = pred {
    //             visitor.push(node);
    //         }
    //     }
    // }

    pub fn reset(&mut self) {
        for node in self.orig_graph.node_weights_mut() {
            node.visible = true;
        }
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

    pub fn get_edge_info(&self, edge_index: EdgeIndex, parser: std::rc::Rc<Z3Parser>, ignore_ids: bool) -> Option<EdgeInfo> {
        if let Some(edge_data) = self.orig_graph.edge_weight(edge_index) {
            let blame_term_idx = edge_data.blame_term_idx.unwrap();
            let blame_term = blame_term_idx.prettify(&parser, ignore_ids);
            let (from, to) = self.orig_graph.edge_endpoints(edge_index).unwrap();
            Some(EdgeInfo { edge_data: *edge_data, blame_term, from, to, })
        } else {
            None
        }
    }

    pub fn get_instantiation_info(&self, node_index: usize, parser: std::rc::Rc<Z3Parser>, ignore_ids: bool) -> Option<InstInfo> {
        let NodeData { inst_idx, .. } = self
            .orig_graph
            .node_weight(NodeIndex::new(node_index))
            .unwrap();
        if let Some(iidx) = inst_idx {
            let inst = parser.instantiations.get(*iidx).unwrap();
            let pretty_blamed_terms = inst
                .blamed_terms
                .iter()
                .map(|term| match term {
                    BlamedTermItem::Single(t) => t.prettify(&parser, ignore_ids),
                    BlamedTermItem::Pair(t1, t2) => format!("{} = {}", t1.prettify(&parser, ignore_ids), t2.prettify(&parser, ignore_ids)),
                })
                .collect::<Vec<String>>();
            let inst_info = InstInfo {
                match_line_no: inst.match_line_no,
                line_no: inst.line_no,
                fingerprint: *inst.fingerprint,
                resulting_term: if let Some(t) = inst.resulting_term {
                    Some(t.prettify(&parser, ignore_ids))
                } else {
                    None
                },
                z3_gen: inst.z3_gen,
                cost: inst.cost,
                quant: inst.quant,
                quant_discovered: inst.quant_discovered,
                formula: inst.quant.prettify(&parser, ignore_ids),
                pattern: if let Some(t) = inst.pattern {
                    Some(t.prettify(&parser, ignore_ids))
                } else {
                    None
                },
                yields_terms: inst.yields_terms.clone().prettify(&parser, ignore_ids),
                bound_terms: inst.bound_terms.clone().prettify(&parser, ignore_ids),
                blamed_terms: pretty_blamed_terms,
                equality_expls: inst.equality_expls.clone().prettify(&parser, ignore_ids),
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
            .filter(|e| if let EdgeType::Direct(_) = e.weight().edge_type { true } else { false })
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
                    branching_rank: 0,
                    min_depth: None,
                    max_depth: 0,
                });
            }
        }
        // then add all edges between nodes
        for dep in &parser.dependencies {
            let from = dep.from;
            if let Some(to) = dep.to {
                if from > 0 {
                    self.add_edge(from, to, dep);
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
        // precompute BFS depth such that we can filter the graph up to some specified depth
        let roots: Vec<NodeIndex> = self
            .orig_graph
            .node_indices()
            .filter(|nx| self.orig_graph.node_weight(*nx).unwrap().parent_count == 0)
            .collect();
        for root in roots {
            let mut bfs = Bfs::new(&self.orig_graph, root);
            while let Some(nx) = bfs.next(&self.orig_graph) {
                let parents = self.orig_graph.neighbors_directed(nx, Incoming);
                let min_parent_depth = parents
                    .filter_map(|parent| self.orig_graph.node_weight(parent).unwrap().min_depth)
                    .min();
                if let Some(depth) = min_parent_depth {
                    self.orig_graph[nx].min_depth = Some(depth + 1);
                } else {
                    // the min_depth is None iff the node at nx has no parents, hence we set the depth to 0
                    self.orig_graph[nx].min_depth = Some(0);
                }
            } 
        }
        // precompute the branching-rank of all nodes by sorting the node_indices by our branching-order
        // in descending order and then assigning the rank to each node
        // Our branching-order is defined as follows:
        // inst_b > inst_a iff (child_count(b) > child_count(a) or (child_count(b) = child_count(a) and line_nr_b < line_nr_a))
        // This is a total order since the line numbers are always guaranteed to be distinct
        // integers.
        let mut branching_ranked_node_indices: Vec<NodeIndex> = self.orig_graph.node_indices().collect();
        let branching_order = |node_a: &NodeIndex, node_b: &NodeIndex| {
            let node_a_data = self.orig_graph.node_weight(*node_a).unwrap();
            let node_b_data = self.orig_graph.node_weight(*node_b).unwrap();
            if node_a_data.child_count < node_b_data.child_count {
                std::cmp::Ordering::Greater
            } else if node_a_data.child_count == node_b_data.child_count
                && node_b_data.line_nr < node_a_data.line_nr
            {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        };
        branching_ranked_node_indices.sort_unstable_by(branching_order);
        for (i, nidx) in branching_ranked_node_indices.iter().enumerate() {
            self.orig_graph.node_weight_mut(*nidx).unwrap().branching_rank = i;
        }
        self.branching_ranked_node_indices = branching_ranked_node_indices;
        // compute the longest distances from root nodes by traversing the graph in topological order
        // and taking max distance among parents + 1. Needed to compute longest paths through selected
        // nodes 
        // root nodes (i.e., nodes without parents) have distance 0
        let mut topo = Topo::new(&self.orig_graph);
        while let Some(node) = topo.next(&self.orig_graph)  {
            let max_parent_depth = self
                .orig_graph
                .neighbors_directed(node, Incoming)
                .map(|nx| self.orig_graph.node_weight(nx).unwrap().max_depth)
                .max();
            if let Some(depth) = max_parent_depth {
                let node_weight = self.orig_graph.node_weight_mut(node).unwrap();
                node_weight.max_depth = depth + 1; 
            }
        }
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
            self.orig_graph.node_weight_mut(node).unwrap().orig_graph_idx = node;
        }
    }

    fn add_edge(&mut self, from: usize, to: usize, dep: &Dependency) {
        if let (Some(&from_node_idx), Some(&to_node_idx)) = (
            self.node_of_line_nr.get(&from),
            self.node_of_line_nr.get(&to),
        ) {
            let edge = self.orig_graph.add_edge(
                from_node_idx,
                to_node_idx,
                EdgeData {
                    edge_type: EdgeType::Direct(dep.dep_type),
                    orig_graph_idx: None,
                    blame_term_idx: dep.blamed,  
                },
            );
            self.orig_graph.edge_weight_mut(edge).unwrap().orig_graph_idx = Some(edge);
        }
    }
}
