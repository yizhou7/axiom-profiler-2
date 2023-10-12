use std::{io::Write, collections::{BTreeSet, HashSet}};
use crate::{file_io::open_file_truncate, items::Dependency};

/// A node representing a quantifier instantiation.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct InstNode {
    line_no: usize,
    quant: String
}

/// Dot representation for each node.
impl std::fmt::Display for InstNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let quant_id = self.quant.replace('#', "");
        write!(f, "{} [ class=\"quant_{}\", tooltip=\"quant_{}\" ]", self.line_no, quant_id, self.quant)
    }
}

/// An edge representing a dependency from one QI to another.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct InstEdge {
    node2: usize,
    node1: usize
}

/// Dot representation for each edge.
impl std::fmt::Display for InstEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {} [ ]", self.node1, self.node2)
    }
}

/// Output Dot representation for entire QI graph.
fn get_dot(name: &str, node_list: &BTreeSet<InstNode>, edge_list: &BTreeSet<InstEdge>) -> String {
    let mut result = String::from("digraph ") + name + " {\n";
    for node in node_list {
        result += &format!("\t{}\n", node);
    }
    for edge in edge_list {
        result += &format!("\t{}\n", edge);
    }
    result += "}\n";
    result

}

/// Filter out theory-solving instantiations
fn filter_theory_inst(_dep: &Dependency) -> bool {
    _dep.quant != "arith#" && _dep.quant != "basic#"
    // true
}

/// Get nodes and edges from dependency list.
fn build_graph(dependencies: &Vec<Dependency>) -> (BTreeSet<InstNode>, BTreeSet<InstEdge>) {
    let filter: fn(&Dependency) -> bool = filter_theory_inst;
    let mut node_list = BTreeSet::new();
    let mut edge_list = BTreeSet::new();
    for dep in dependencies {
        if filter(dep) {
            if dep.from != 0 {
                edge_list.insert(InstEdge { node1: dep.from, node2: dep.to });
            }
            node_list.insert(InstNode { line_no: dep.to, quant: dep.quant.clone() });
        }
    }
    (node_list, edge_list)
}

/// Filter `node_list` and `edge_list` so that only the earliest instantiations are displayed.
fn filter_graph_line_no(node_list: BTreeSet<InstNode>, edge_list: BTreeSet<InstEdge>) -> (BTreeSet<InstNode>, BTreeSet<InstEdge>) {
    const FILTER_LIMIT: usize = 250;    // TODO: make this a setting
    let num_nodes_to_get = std::cmp::min(FILTER_LIMIT, node_list.len());
    let node_list_result = Vec::from_iter(node_list)[..num_nodes_to_get].to_vec();
    let edge_list_result = Vec::from_iter(edge_list);

    // finds "index" of the dependency (edge) with the last included instantiation line number
    // uses property that instantiations are sorted by "to" inst ID
    // include all dependencies up to that one
    let index = edge_list_result.iter()
    .map(|e| e.node2)
    .collect::<Vec<usize>>()
    .binary_search(&node_list_result.last().unwrap().line_no) // unwrap can fail
    .unwrap_or_else(|index| index);
    let index = std::cmp::min(index + 1, edge_list_result.len());
    (BTreeSet::from_iter(node_list_result), BTreeSet::from_iter(edge_list_result[..index].to_owned()))
}

/// Write Dot string of instantiation graph to file, and then a CSS string for quantifier colors.
pub fn output_dot_and_css_to_file(dot_filename: &str, css_filename: &str, dependencies: &Vec<Dependency>) {
    let node_list = output_dot_to_file(dot_filename, dependencies);
    output_css_to_file(css_filename, &node_list);
}

pub fn get_dot_output_as_string(dependencies: &Vec<Dependency>) -> String {
    let (node_list, edge_list) = build_graph(dependencies);
    let (node_list, edge_list) = filter_graph_line_no(node_list, edge_list);
    get_dot("instantiations", &node_list, &edge_list)
}

// Turn `dependencies` into a Dot file
fn output_dot_to_file(filename: &str, dependencies: &Vec<Dependency>) -> BTreeSet<InstNode> {
    let mut file = open_file_truncate(filename);
    let (node_list, edge_list) = build_graph(dependencies);
    let (node_list, edge_list) = filter_graph_line_no(node_list, edge_list);
    file.write_all(&get_dot("instantiations", &node_list, &edge_list).into_bytes()).expect("failed to write dot file");
    node_list
}

/// Build CSS file from `node_list`
fn output_css_to_file(filename: &str, node_list: &BTreeSet<InstNode>) {
    use crate::css::make_css;
    let mut file = open_file_truncate(filename);
    let quant_set: HashSet<String> = node_list.iter()
    .map(|node| String::from("quant_") + &node.quant.replace('#', ""))
    .collect();
    let css = make_css::make_css_string(&quant_set);
    file.write_all(&css.into_bytes()).expect("failed to write CSS file");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_display() {
        let node1 = InstNode {
            line_no: 1,
            quant: "#A".to_string()
        };
        let node2 = InstNode {
            line_no: 2,
            quant: "ns#B".to_string()
        };
        assert_eq!(&format!("{}", node1), "1 [ class=\"quant_A\", tooltip=\"quant_#A\" ]");
        assert_eq!(&format!("{}", node2), "2 [ class=\"quant_nsB\", tooltip=\"quant_ns#B\" ]");
    }

    #[test]
    fn test_edge_display() {
        let edge = InstEdge {
            node1: 100,
            node2: 200
        };
        assert_eq!(&format!("{}", edge), "100 -> 200 [ ]");
    }

    #[test]
    fn test_get_dot() {
        let node1 = InstNode {
            line_no: 1,
            quant: "#A".to_string()
        };
        let node2 = InstNode {
            line_no: 2,
            quant: "ns#B".to_string()
        };
        let edge = InstEdge {
            node1: 1,
            node2: 2
        };
        let nodes = BTreeSet::from([node1, node2]);
        let edges = BTreeSet::from([edge]);
        assert_eq!(&get_dot("test", &nodes, &edges), 
        r#"digraph test {
	1 [ class="quant_A", tooltip="quant_#A" ]
	2 [ class="quant_nsB", tooltip="quant_ns#B" ]
	1 -> 2 [ ]
}
"#);
    }
}