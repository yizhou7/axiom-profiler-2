use crate::{
    input_state::{InputValue, UsizeInput},
    toggle_switch::ToggleSwitch, svg_result::Filter,
};
use smt_log_parser::parsers::z3::results::FilterSettings;
use yew::prelude::*;
use yew_hooks::prelude::*;

#[derive(Properties, PartialEq)]
pub struct GraphFilterProps {
    pub title: AttrValue,
    pub update_settings: Callback<Filter>,
    pub dependency: AttrValue,
}

#[function_component(GraphFilter)]
pub fn graph_filter(props: &GraphFilterProps) -> Html {
    let max_line_nr = use_reducer(InputValue::default);
    let exclude_theory_inst = use_bool_toggle(true);
    let max_instantiations = use_reducer(InputValue::default);

    // let update_settings = {
    //     let max_line_nr = max_line_nr.clone();
    //     let exclude_theory_inst = exclude_theory_inst.clone();
    //     let max_instantiations = max_instantiations.clone();
    //     let callback = props.update_settings.clone();
    //     // props.update_settings.reform(move |_| FilterSettings {
    //     //     max_line_nr: max_line_nr.value,
    //     //     exclude_theory_inst: *exclude_theory_inst,
    //     //     max_instantiations: max_instantiations.value,
    //     // })
    //     Callback::from(move |_| {
    //         callback.emit(FilterSettings {
    //             max_line_nr: max_line_nr.value,
    //             exclude_theory_inst: *exclude_theory_inst,
    //             max_instantiations: max_instantiations.value,
    //         })
    //     })
    // };

    let add_max_line_nr_filter = {
        let max_line_nr = max_line_nr.clone();
        let callback = props.update_settings.clone();
        Callback::from(move |_| {
            callback.emit(Filter::MaxLineNr(max_line_nr.value))
        })
    };
    let add_theory_filter = {
        let exclude_theory_inst = exclude_theory_inst.clone();
        let callback = props.update_settings.clone();
        Callback::from(move |_| {
            callback.emit(Filter::IgnoreTheorySolving(*exclude_theory_inst))
        })
    };
    let add_max_insts_filter = {
        let max_instantiations = max_instantiations.clone();
        let callback = props.update_settings.clone();
        Callback::from(move |_| {
            callback.emit(Filter::MaxInsts(max_instantiations.value))
        })
    };
    html! {
        <div>
            <h2>{"Add (optional) filters:"}</h2>
            <UsizeInput
                label={"Render graph up to line number "}
                dependency={props.dependency.clone()}
                input_value={max_line_nr}
                default_value={usize::MAX}
            />
            <button onclick={add_max_line_nr_filter}>{"Add"}</button>
            <ToggleSwitch
                label={"Ignore theory-solving instantiations: "}
                dependency={props.dependency.clone()}
                input_value={exclude_theory_inst}
            />
            <button onclick={add_theory_filter}>{"Add"}</button>
            <UsizeInput
                label={"Render the n most expensive instantiations where n = "}
                dependency={props.dependency.clone()}
                input_value={max_instantiations}
                default_value={250}
            />
            <button onclick={add_max_insts_filter}>{"Add"}</button>
            // <SelectDropDown
            //     label={"Select a layout engine:"}
            //     options={layout_engines}
            //     selected_option={layout_engine}
            // />
            // <button onclick={update_settings}>{"Render graph"}</button>
        </div>

    }
}
