use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SelectDropDownProps where {
    pub label: AttrValue,
    pub options: Vec<AttrValue>,
    pub selected_option:  UseStateHandle<AttrValue>,
}

#[function_component(SelectDropDown)]
pub fn select_dropdown(props: &SelectDropDownProps) -> Html {
    let options = props.options.iter().enumerate().map(|(idx, option)| html! {
        <option value={props.options[idx].clone()}>{option.to_string()}</option>
    });

    // let change_selected_option = Callback::from({
    //     let selected_option = props.selected_option.clone();
    //     move |event: Event| {
    //         if let Some(target) = event.target() {
    //             if let Ok(select) = target.dyn_into::<HtmlSelectElement>() {
    //                 let selected_value = select.value().to_string();
    //                 log::debug!("Selected: {selected_value}");
    //             }
    //         }
    //     }
    // });

    html! {
        <div>
            <label for="my-select">{props.label.to_string()}</label>
            // <select id="my-select" onchange={change_selected_option}>
            <select id="my-select" >
            // <select id="my-select" >
                {for options}
            </select>
        </div>
    }
}