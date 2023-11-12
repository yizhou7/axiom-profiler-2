use yew::prelude::*;
use wasm_bindgen::{prelude::Closure};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub svg_text: AttrValue,
    pub update_selected_node: Callback<bool>,
}

#[function_component(Graph)]
pub fn graph(props: &GraphProps) -> Html {
    // let graph_state = use_reducer(GraphState::default);
    let svg_result = Html::from_html_unchecked(props.svg_text.clone());
    let div_ref = use_node_ref();

    {
        // Whenever SVG text changes, need to attach event listeners to new nodes 
        let div_ref = div_ref.clone();
        let svg_text = props.svg_text.clone();
        let callback = props.update_selected_node.clone();

        use_effect_with(svg_text, move |_| {
            web_sys::console::log_1(&"Using effect".into());
            let div = div_ref
                .cast::<HtmlElement>()
                .expect("div_ref not attached to div element");

            let closure: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
                callback.emit(true);
            });

            // Iterate through all nodes with class "node" and add a click event listener
            let descendant_nodes = div.get_elements_by_class_name("node");
            for i in 0..descendant_nodes.length() {
                if let Some(node) = descendant_nodes.item(i) {
                    let title_element = node.query_selector("title").expect("Failed to select title element");
                    if let Some(title) = title_element {
                        let title_content = title.text_content();
                    } 
                    // node.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref()).unwrap();
                    node.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                }
            }

            move || {
                // Remove event listeners when the component is unmounted
                for i in 0..descendant_nodes.length() {
                    if let Some(node) = descendant_nodes.item(i) {
                        node.remove_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
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