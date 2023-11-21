use petgraph::Direction::{Incoming, Outgoing};
use smt_log_parser::parsers::z3::inst_graph::InstInfo;
use yew::prelude::*;

use crate::graph_filters::Filter;

#[derive(Properties, PartialEq)]
pub struct SelectedNodeProps {
    pub selected_node: InstInfo,
    pub action: Callback<Filter>,
}

#[function_component(SelectedNode)]
pub fn selected_node(props: &SelectedNodeProps) -> Html {
    let selected_inst = props.selected_node.clone();
    let hide_node = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Filter::Hide(selected_inst.node_index));
        })
    };
    let show_children = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Filter::ShowNeighbours(selected_inst.node_index, Outgoing))
        })
    };
    let show_parents = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Filter::ShowNeighbours(selected_inst.node_index, Incoming))
        })
    };
    let show_source_tree = {
        let callback = props.action.clone();
        Callback::from(move |_| callback.emit(Filter::ShowSourceTree(selected_inst.node_index)))
    };
    let selected_node_text = format!(
        "You selected node {}. Here are available actions: ",
        selected_inst.node_index.index()
    );
    html! {
    <div>
        <h4>{selected_node_text}</h4>
        <button onclick={hide_node}>{"Hide"}</button>
        <button onclick={show_children}>{"Show children"}</button>
        <button onclick={show_parents}>{"Show parents"}</button>
        <button onclick={show_source_tree}>{"Only show ancestors"}</button>
        <h2>{"Information about selected node:"}</h2>
        <ul>
            <li><h4>{"Instantiation happens at line number: "}</h4><p>{selected_inst.line_no}</p></li>
            <li><h4>{"Cost: "}</h4><p>{selected_inst.cost}</p></li>
            <li><h4>{"Instantiated formula: "}</h4><p>{selected_inst.formula.clone()}</p></li>
            // <li><h4>{"Bound terms: "}</h4>{for &inst_info.bound_terms}</li>
            // <li><h4>{"Yield terms: "}</h4>{for &inst_info.yields_terms}</li>
            // <li><h4>{"Variable binding information: "}</h4></li>
            // <li><h4>{"Involved equalities: "}</h4></li>
        </ul>
    </div>
    }
}
