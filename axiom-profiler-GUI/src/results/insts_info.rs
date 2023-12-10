use yew::{prelude::*, virtual_dom::VNode};
use smt_log_parser::{parsers::z3::inst_graph::{InstInfo, EdgeInfo}, items::DepType};
use petgraph::graph::NodeIndex;
use smt_log_parser::parsers::z3::inst_graph::EdgeType;

#[derive(Properties, PartialEq)]
pub struct InstsInfoProps {
    pub selected_nodes: Vec<InstInfo>,
    pub selected_edges: Vec<(NodeIndex, NodeIndex, EdgeInfo)>,
    pub last_selected_node: Option<NodeIndex>,
}

#[function_component(InstsInfo)]
pub fn insts_info(props: &InstsInfoProps) -> Html {
    let selected_nodes_info: Vec<VNode> = props
        .selected_nodes 
        .iter()
        .map(|selected_inst| { 
            let get_ul = |label: &str, items: &Vec<String>| html! {
                <>
                    <h4>{label}</h4>
                    <ul>{for items.iter().map(|item| html!{<li>{item}</li>})}</ul>
                </>
            };
            let open = if let Some(node) = props.last_selected_node {
                node == selected_inst.node_index
            } else {
                false
            };
            html! {
            <details {open}>
                <summary>{format!("Node {}", selected_inst.node_index.index())}</summary>
                <ul>
                    <li><h4>{"Instantiation happens at line number: "}</h4><p>{if let Some(val) = selected_inst.line_no {format!("{val}")} else { String::new() }}</p></li>
                    <li><h4>{"Cost: "}</h4><p>{selected_inst.cost}</p></li>
                    <li><h4>{"Instantiated formula: "}</h4><p>{&selected_inst.formula}</p></li>
                    <li>{get_ul("Blamed terms: ", &selected_inst.blamed_terms)}</li>
                    <li>{get_ul("Bound terms: ", &selected_inst.bound_terms)}</li>
                    <li>{get_ul("Yield terms: ", &selected_inst.yields_terms)}</li>
                    <li>{get_ul("Equality explanations: ", &selected_inst.equality_expls)}</li>
                    <li><h4>{"Resulting term: "}</h4><p>{if let Some(ref val) = selected_inst.resulting_term {format!("{val}")} else { String::new() }}</p></li>
                </ul>
            </details>
        }})
        .collect();
    let selected_edges_info: Vec<VNode> = props
        .selected_edges
        .iter()
        .map(|(from, to, data)| {
            html! {
            <details>
                <summary>{format!("Dependency from {} to {}", from.index(), to.index())}</summary>
                {match data.edge_data.edge_type {
                    EdgeType::Direct(DepType::Term) => html! {
                        <div>
                        <h4>{"Blame term: "}</h4><p>{data.blame_term.clone()}</p>
                        </div>
                    }, 
                    EdgeType::Direct(DepType::Equality) => html! {
                        <div>
                        <h4>{"Equality: "}</h4><p>{data.blame_term.clone()}</p>
                        </div>
                    },
                    _ => html! {} 
                }}
            </details>
            }
        })
        .collect();
    html! {
        <>
            <h2>{"Information about selected nodes:"}</h2>
            { for selected_nodes_info }
            <h2>{"Information about selected dependencies:"}</h2>
            { for selected_edges_info }
        </>
    }
}