use yew::prelude::*;
use crate::svg_result::GraphState;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[function_component(Graph)]
pub fn graph() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    let svg_attr_val = AttrValue::from(state.svg_text);
    let svg_result = Html::from_html_unchecked(svg_attr_val);
    let div_ref = use_node_ref();
    {
        let div_ref = div_ref.clone();

        use_effect_with(div_ref, |div_ref| {
            let div = div_ref
                .cast::<HtmlElement>()
                .expect("div_ref not attached to div element");

            let listener = Closure::<dyn Fn(Event)>::wrap(Box::new(|_| {
                    web_sys::console::log_1(&"Clicked!".into());
                }));

            // Iterate through all nodes with class "node" and add a click event listener
            let descendant_nodes = div.get_elements_by_class_name("node");
            for i in 0..descendant_nodes.length() {
                if let Some(node) = descendant_nodes.item(i) {
                    web_sys::console::log_1(&"Adding event listener".into());
                    node.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref()).unwrap();
                }
            }

            move || {
                // Remove event listeners when the component is unmounted
                for i in 0..descendant_nodes.length() {
                    if let Some(node) = descendant_nodes.item(i) {
                        web_sys::console::log_1(&"Removing event listener".into());
                        node.remove_event_listener_with_callback("click", listener.as_ref().unchecked_ref()).unwrap();
                    }
                }
            }
        });
    }

    html! {
        <div ref={div_ref}>
            {svg_result}
        </div>
    }
}