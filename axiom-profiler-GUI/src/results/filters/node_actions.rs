use petgraph::Direction::{Incoming, Outgoing};
use smt_log_parser::parsers::z3::inst_graph::InstInfo;
use yew::{prelude::*, virtual_dom::VNode};

use super::graph_filters::Filter;

#[derive(Properties, PartialEq)]
pub struct NodeActionsProps {
    pub selected_nodes: Vec<InstInfo>,
    pub action: Callback<Vec<Filter>>,
}

#[function_component(NodeActions)]
pub fn node_actions(props: &NodeActionsProps) -> Html {
    let callback_from = |filter_for_inst: Box<dyn Fn(&InstInfo) -> Filter>| {
        let callback = props.action.clone();
        let selected_insts = props.selected_nodes.clone();
        Callback::from(move |_| {
            let filters: Vec<Filter> = selected_insts
                .iter()
                .map(&filter_for_inst)
                .collect();
            callback.emit(filters);
        })
    };
    let show_subtree = callback_from(Box::new(|inst: &InstInfo| Filter::VisitSubTreeWithRoot(inst.node_index, true)));
    let hide_subtree = callback_from(Box::new(|inst: &InstInfo| Filter::VisitSubTreeWithRoot(inst.node_index, false)));
    let show_children = callback_from(Box::new(|inst: &InstInfo| Filter::ShowNeighbours(inst.node_index, Outgoing)));
    let show_parents = callback_from(Box::new(|inst: &InstInfo| Filter::ShowNeighbours(inst.node_index, Incoming)));
    let show_source_tree = callback_from(Box::new(|inst: &InstInfo| Filter::VisitSourceTree(inst.node_index, true)));
    let hide_source_tree = callback_from(Box::new(|inst: &InstInfo| Filter::VisitSourceTree(inst.node_index, false)));
    let ignore_quantifier = callback_from(Box::new(|inst: &InstInfo| Filter::IgnoreQuantifier(inst.quant)));
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
    html! {
    <>
        <h4>{selected_nodes_text}</h4>
        <div>
            <button onclick={show_subtree}>{"Show subtree with this root"}</button>
            <button onclick={hide_subtree}>{"Hide subtree with this root"}</button>
        </div>
        <div>
            <button onclick={show_children}>{"Show children"}</button>
            <button onclick={show_parents}>{"Show parents"}</button>
        </div>
        <div>
            <button onclick={show_source_tree}>{"Show ancestors"}</button>
            <button onclick={hide_source_tree}>{"Hide ancestors"}</button>
        </div>
        <button onclick={ignore_quantifier}>{"Ignore all nodes of this quantifier"}</button>
    </>
    }
}