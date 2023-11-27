use petgraph::Direction::{Incoming, Outgoing};
use smt_log_parser::parsers::z3::inst_graph::InstInfo;
use yew::{prelude::*, virtual_dom::VNode};

use super::graph_filters::Filter;

#[derive(Properties, PartialEq)]
pub struct SelectedNodeProps {
    pub selected_nodes: Vec<InstInfo>,
    pub action: Callback<Filter>,
}

#[function_component(SelectedNode)]
pub fn selected_node(props: &SelectedNodeProps) -> Html {
    let hide_node = {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| {
            for inst in &selected_insts {
                callback.emit(Filter::Hide(inst.node_index));
            }
        })
    };
    let show_children = {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| {
            for inst in &selected_insts {
                callback.emit(Filter::ShowNeighbours(inst.node_index, Outgoing))
            }
        })
    };
    let show_parents = {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| {
            for inst in &selected_insts {
                callback.emit(Filter::ShowNeighbours(inst.node_index, Incoming))
            }
        })
    };
    let show_source_tree = {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| {
            for inst in &selected_insts {
                callback.emit(Filter::ShowSourceTree(inst.node_index))
            }
        })
    };
    let ignore_quantifier = {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| { 
            for inst in &selected_insts {
                callback.emit(Filter::IgnoreQuantifier(inst.quant_idx))
            }
        })
    };
    let mut selected_nodes = props
        .selected_nodes
        .iter()
        .map(|inst| format!("{}", inst.node_index.index()))
        .collect::<Vec<String>>()
        .join(", ");
    selected_nodes.push('.');
    let selected_nodes_text = format!(
        "You selected node(s) {} Here are available actions: ",
        selected_nodes
    );
    let selected_nodes_info: Vec<VNode> = props
        .selected_nodes 
        .iter()
        .map(|selected_inst| { html! {
            <details>
            <summary>{format!("Node {}", selected_inst.node_index.index())}</summary>
            <ul>
                <li><h4>{"Instantiation happens at line number: "}</h4><p>{selected_inst.line_no}</p></li>
                <li><h4>{"Cost: "}</h4><p>{selected_inst.cost}</p></li>
                <li><h4>{"Instantiated formula: "}</h4><p>{selected_inst.formula.clone()}</p></li>
                // <li><h4>{"Bound terms: "}</h4>{for &inst_info.bound_terms}</li>
                // <li><h4>{"Yield terms: "}</h4>{for &inst_info.yields_terms}</li>
                // <li><h4>{"Variable binding information: "}</h4></li>
                // <li><h4>{"Involved equalities: "}</h4></li>
            </ul>
        </details>
        }}).collect();
    html! {
    // <div>
    <>
        <h4>{selected_nodes_text}</h4>
        <button onclick={hide_node}>{"Hide"}</button>
        <button onclick={show_children}>{"Show children"}</button>
        <button onclick={show_parents}>{"Show parents"}</button>
        <button onclick={show_source_tree}>{"Only show ancestors"}</button>
        <button onclick={ignore_quantifier}>{"Ignore all nodes of this quantifier"}</button>
        <h2>{"Information about selected nodes:"}</h2>
        { for selected_nodes_info }
        
    // </div>
    </>
    }
}
