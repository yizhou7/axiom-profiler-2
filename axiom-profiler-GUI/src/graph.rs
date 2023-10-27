use yew::prelude::*;
use crate::svg_result::GraphState;
use wasm_bindgen::{prelude::Closure, JsValue};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[function_component(Graph)]
pub fn graph() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    let svg_attr_val = AttrValue::from(state.svg_text);
    let svg_result = Html::from_html_unchecked(svg_attr_val);
    let div_ref = use_node_ref();
    let max_line_nr = state.max_line_nr;
    {
        let div_ref = div_ref.clone();

        use_effect_with(div_ref, |div_ref| {
            web_sys::console::log_1(&"Using effect".into());
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
    {
        let div_ref = div_ref.clone();
        let max_line_nr = max_line_nr.clone();

        use_effect_with(max_line_nr, move |&max_line_nr| {
            web_sys::console::log_1(&"Using effect due to change in max_line_nr: ".into());
            let max_line_nr_js = JsValue::from(max_line_nr);
            web_sys::console::log_1(&max_line_nr_js);
            let div = div_ref
                .cast::<HtmlElement>()
                .expect("div_ref not attached to div element");

            // Iterate through all nodes with class "node" and add a click event listener
            let descendant_nodes = div.get_elements_by_class_name("node");
            for i in 0..descendant_nodes.length() {
                web_sys::console::log_1(&"Iterating through nodes".into());
                if let Some(node) = descendant_nodes.item(i) {
                    let text_element = node.query_selector("text").expect("Failed to select text element");
                    if let Some(text) = text_element {
                        if let Some(text_content) = text.text_content() {
                            match text_content.parse::<i32>() {
                                Ok(line_nr) => if line_nr > max_line_nr {
                                    web_sys::console::log_1(&"Hiding node".into());
                                    node.set_attribute("visibility", "hidden");
                                } else {
                                    web_sys::console::log_1(&"Showing node".into());
                                    node.set_attribute("visibility", "visible");
                                },
                                Err(_) => { 
                                    web_sys::console::log_1(&"Showing node".into());
                                    node.set_attribute("visibility", "visible"); }
                            }
                        }
                        
                    }
                }
            }
            web_sys::console::log_1(&"Finished iterating through nodes".into());
            move || {
                web_sys::console::log_1(&"Tearing down".into());
            }
        });
    }

    html! {
        <div ref={div_ref}>
            {svg_result}
        </div>
    }
}