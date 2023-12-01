use super::selected_node::SelectedNode;
use crate::utils::input_state::{InputValue, UsizeInput};
use petgraph::{stable_graph::NodeIndex, Direction};
use smt_log_parser::{
    items::QuantIdx,
    parsers::z3::inst_graph::{InstGraph, InstInfo, NodeData},
};
use std::fmt::Display;
use yew::prelude::*;

#[derive(Clone, Copy)]
pub enum Filter {
    MaxNodeIdx(usize),
    IgnoreTheorySolving,
    IgnoreQuantifier(QuantIdx),
    MaxInsts(usize),
    Hide(NodeIndex),
    ShowNeighbours(NodeIndex, Direction),
    ShowSourceTree(NodeIndex),
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxNodeIdx(node_idx) => write!(f, "Only show nodes up to index {}", node_idx),
            Self::IgnoreTheorySolving => write!(f, "Ignore theory solving instantiations"),
            Self::IgnoreQuantifier(qidx) => {
                write!(f, "Ignore instantiations of quantifier {}", qidx)
            }
            Self::MaxInsts(max) => write!(f, "Show the {} most expensive instantiations", max),
            Self::Hide(nidx) => write!(f, "Hide node {} and its descendants", nidx.index()),
            Self::ShowSourceTree(nidx) => {
                write!(f, "Only show the ancestors of node {}", nidx.index())
            }
            Self::ShowNeighbours(nidx, direction) => match direction {
                Direction::Incoming => write!(f, "Show the parents of node {}", nidx.index()),
                Direction::Outgoing => write!(f, "Show the children of node {}", nidx.index()),
            },
        }
    }
}

impl Filter {
    pub fn apply(self: Filter, graph: &mut InstGraph) {
        match self {
            Filter::MaxNodeIdx(max) => {
                graph.retain_nodes(|node: &NodeData| usize::from(node.orig_graph_idx.index()) <= max)
            }
            Filter::IgnoreTheorySolving => {
                graph.retain_nodes(|node: &NodeData| !node.is_theory_inst)
            }
            Filter::IgnoreQuantifier(qidx) => {
                graph.retain_nodes(|node: &NodeData| node.quant_idx != qidx)
            },
            Filter::MaxInsts(n) => graph.keep_n_most_costly(n),
            _ => (),
            // Filter::Hide(nidx) => graph.remove_subtree_with_root(nidx),
            // Filter::ShowNeighbours(nidx, direction) => graph.show_neighbours(nidx, direction),
            // Filter::ShowSourceTree(nidx) => graph.only_show_ancestors(nidx),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct GraphFilterProps {
    pub add_filter: Callback<Filter>,
    pub dependency: AttrValue,
}

#[function_component(GraphFilter)]
pub fn graph_filter(props: &GraphFilterProps) -> Html {
    let max_node_idx = use_reducer(InputValue::default);
    let max_instantiations = use_reducer(InputValue::default);
    let selected_inst = use_context::<Vec<InstInfo>>().expect("no ctx found");

    let add_max_line_nr_filter = {
        let max_node_idx = max_node_idx.clone();
        let callback = props.add_filter.clone();
        Callback::from(move |_| callback.emit(Filter::MaxNodeIdx(max_node_idx.value)))
    };
    let add_theory_filter = {
        let callback = props.add_filter.clone();
        Callback::from(move |_| callback.emit(Filter::IgnoreTheorySolving))
    };
    let add_max_insts_filter = {
        let max_instantiations = max_instantiations.clone();
        let callback = props.add_filter.clone();
        Callback::from(move |_| callback.emit(Filter::MaxInsts(max_instantiations.value)))
    };
    html! {
        <div>
            <h2>{"Add (optional) filters:"}</h2>
            <div>
                <UsizeInput
                    label={"Only show nodes up to index "}
                    dependency={props.dependency.clone()}
                    input_value={max_node_idx}
                    default_value={usize::MAX}
                    placeholder={""}
                />
                <button onclick={add_max_line_nr_filter}>{"Add"}</button>
            </div>
            <div>
                <label for="theory_button">{"Ignore theory-solving instantiations"}</label>
                <button onclick={add_theory_filter} id="theory_button">{"Add"}</button>
            </div>
            <div>
                <UsizeInput
                    label={"Render the n most expensive instantiations where n = "}
                    dependency={props.dependency.clone()}
                    input_value={max_instantiations}
                    default_value={super::super::svg_result::DEFAULT_NODE_COUNT}
                    placeholder={""}
                />
                <button onclick={add_max_insts_filter}>{"Add"}</button>
            </div>
            {if selected_inst.len() > 0 {
                html! {
                    <SelectedNode selected_nodes={selected_inst} action={props.add_filter.clone()} />
                }
            } else {
                html! {}
            }}
            // <SelectDropDown
            //     label={"Select a layout engine:"}
            //     options={layout_engines}
            //     selected_option={layout_engine}
            // />
            // <button onclick={update_settings}>{"Render graph"}</button>
        </div>

    }
}
