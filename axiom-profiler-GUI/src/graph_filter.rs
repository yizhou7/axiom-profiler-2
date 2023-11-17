use crate::{
    input_state::{InputValue, UsizeInput}, filter_chain::Filter,
};
use yew::prelude::*;

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
            <UsizeInput
                label={"Render graph up to line number "}
                dependency={props.dependency.clone()}
                input_value={max_line_nr}
                default_value={usize::MAX}
            />
            <button onclick={add_max_line_nr_filter}>{"Add"}</button>
            <p>{"Ignore theory-solving instantiations"}</p>
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
