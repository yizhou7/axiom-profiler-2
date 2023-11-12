use yew::prelude::*;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub svg_text: AttrValue,
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

        use_effect_with(svg_text, move |_| {
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
                    node.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref()).unwrap();
                }
            }

            move || {
                // Remove event listeners when the component is unmounted
                for i in 0..descendant_nodes.length() {
                    if let Some(node) = descendant_nodes.item(i) {
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