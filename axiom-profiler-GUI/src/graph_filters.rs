use std::fmt::Display;
use crate::input_state::{InputValue, UsizeInput};
use smt_log_parser::parsers::z3::inst_graph::{InstGraph, NodeData};
use yew::prelude::*;

#[derive(Clone, Copy)]
pub enum Filter {
    MaxLineNr(usize),
    IgnoreTheorySolving,
    MaxInsts(usize),
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxLineNr(line_nr) => write!(f, "Only show up to line number {}", line_nr),
            Self::IgnoreTheorySolving => write!(f, "Ignore theory solving instantiations"),
            Self::MaxInsts(max) => write!(f, "Show the {} most expensive instantiations", max)
        }
    }
}

impl Filter {
    pub fn apply(self: Filter, graph: &mut InstGraph) {
        match self {
            Filter::MaxLineNr(max_line_nr) => {
                graph.retain_nodes(|node: &NodeData| node.line_nr <= max_line_nr)
            }, 
            Filter::IgnoreTheorySolving => {
                graph.retain_nodes_and_reconnect(|node: &NodeData| !node.is_theory_inst)
            },
            Filter::MaxInsts(n) => {
                graph.keep_n_most_costly(n);
            }
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
    let max_line_nr = use_reducer(InputValue::default);
    let max_instantiations = use_reducer(InputValue::default);

    let add_max_line_nr_filter = {
        let max_line_nr = max_line_nr.clone();
        let callback = props.add_filter.clone();
        Callback::from(move |_| {
            callback.emit(Filter::MaxLineNr(max_line_nr.value))
        })
    };
    let add_theory_filter = {
        let callback = props.add_filter.clone();
        Callback::from(move |_| {
            callback.emit(Filter::IgnoreTheorySolving)
        })
    };
    let add_max_insts_filter = {
        let max_instantiations = max_instantiations.clone();
        let callback = props.add_filter.clone();
        Callback::from(move |_| {
            callback.emit(Filter::MaxInsts(max_instantiations.value))
        })
    };
    html! {
        <div>
            <h2>{"Add (optional) filters:"}</h2>
            <div>
                <UsizeInput
                    label={"Render graph up to line number "}
                    dependency={props.dependency.clone()}
                    input_value={max_line_nr}
                    default_value={usize::MAX}
                />
                <button onclick={add_max_line_nr_filter}>{"Add"}</button>
            </div>
            <div>
                <p>{"Ignore theory-solving instantiations"}</p>
                <button onclick={add_theory_filter}>{"Add"}</button>
            </div>
            <div>
                <UsizeInput
                    label={"Render the n most expensive instantiations where n = "}
                    dependency={props.dependency.clone()}
                    input_value={max_instantiations}
                    default_value={125}
                />
                <button onclick={add_max_insts_filter}>{"Add"}</button>
            </div>
            // <SelectDropDown
            //     label={"Select a layout engine:"}
            //     options={layout_engines}
            //     selected_option={layout_engine}
            // />
            // <button onclick={update_settings}>{"Render graph"}</button>
        </div>

    }
}
