use petgraph::Direction::{self, Outgoing, Incoming};
use smt_log_parser::parsers::z3::inst_graph::InstInfo;
use yew::prelude::*;

pub enum Action {
    Hide,
    ShowNeighbours(Direction),
    ShowSourceTree,
}

#[derive(Properties, PartialEq)]
pub struct SelectedNodeProps {
    pub selected_inst: Option<InstInfo>,
    pub action: Callback<Action>,
}

#[function_component(SelectedNode)]
pub fn selected_node(props: &SelectedNodeProps) -> Html {
    let selected_inst = props.selected_inst.clone();
    let hide_node = { 
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Action::Hide);
        })
    };
    let show_children = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Action::ShowNeighbours(Outgoing))
        })
    };
    let show_parents = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Action::ShowNeighbours(Incoming))
        })
    };
    let show_source_tree = {
        let callback = props.action.clone();
        Callback::from(move |_| {
            callback.emit(Action::ShowSourceTree)
        })
    };
    if let Some(inst_info) = selected_inst {
        html! {
        <div style="width: 50%; float: left;">
            <h2>{"Information about selected node:"}</h2>
            <ul>
                <li><h4>{"Instantiation happens at line number: "}</h4><p>{inst_info.line_no}</p></li>
                <li><h4>{"Cost: "}</h4><p>{inst_info.cost}</p></li>
                <li><h4>{"Instantiated formula: "}</h4><p>{inst_info.formula.clone()}</p></li>
                // <li><h4>{"Bound terms: "}</h4>{for &inst_info.bound_terms}</li>
                // <li><h4>{"Yield terms: "}</h4>{for &inst_info.yields_terms}</li>
                // <li><h4>{"Variable binding information: "}</h4></li>
                // <li><h4>{"Involved equalities: "}</h4></li>
            </ul>
            <button onclick={hide_node}>{"Hide selected node and its descendants"}</button>
            <button onclick={show_children}>{"Show children of selected node"}</button>
            <button onclick={show_parents}>{"Show parents of selected node"}</button>
            <button onclick={show_source_tree}>{"Only show ancestors of selected node"}</button>
        </div>
        }
    } else {
        html! {}
    }
}