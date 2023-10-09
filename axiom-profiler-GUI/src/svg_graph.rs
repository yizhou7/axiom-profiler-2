use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use prototype::items::Dependency;

fn create_petgraph(dependencies: &Vec<Dependency>) -> DiGraph<usize,()> {
    let not_theory_inst = |dep: &Dependency| -> bool {dep.quant != "arith#" && dep.quant != "basic#"};
    let mut g = DiGraph::<usize,()>::new();
    for dep in dependencies {
        if not_theory_inst(dep) {
            let from_node = g.add_node(dep.to);
            if dep.from != 0 {
                // has incoming edge/dependency on other instantiation
                let to_node = g.add_node(dep.from);
                g.add_edge(from_node, to_node, ());
            }
        }
    }
    g
}

pub fn create_svg_graph(dependencies: &Vec<Dependency>) -> String {
    let graph = create_petgraph(dependencies);
    // returns a vector of all NodeIndices. The index of a node in sorted_nodes corresponds to its topological order
    let sorted_nodes = match toposort(&graph, None) {
        Ok(sorted_nodes) => sorted_nodes,
        Err(_) => panic!(),
    };
    String::new()
} 