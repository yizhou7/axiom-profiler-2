use yew::prelude::*;
use wasm_bindgen::{prelude::Closure};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub svg_text: AttrValue,
    pub update_selected_node: Callback<String>,
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

            // construct event_listeners that emit node indices (contained in title tags)
            let descendant_nodes = div.get_elements_by_class_name("node");
            let closures: Vec<Closure<dyn Fn(Event)>> = (0..descendant_nodes.length())
                .map(|i| {
                    let node = descendant_nodes.item(i).unwrap(); 
                    let title_element = node.query_selector("title").expect("Failed to select title element").unwrap();
                    let title_content = title_element.text_content().unwrap();
                    let callback = callback.clone();
                    let closure: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
                        callback.emit(title_content.clone());
                    });
                    closure
                })
                .collect(); 

            // attach event listeners to the nodes
            for i in 0..descendant_nodes.length() {
                if let Some(node) = descendant_nodes.item(i) {
                    let closure = closures.as_slice()[i as usize].as_ref();
                    node.add_event_listener_with_callback("click", closure.unchecked_ref()).unwrap();
                }
            }

            move || {
                // Remove event listeners when the component is unmounted
                for i in 0..closures.len() {
                    if let Some(node) = descendant_nodes.item(i as u32) {
                        let closure = closures.as_slice()[i as usize].as_ref();
                        node.remove_event_listener_with_callback("click", closure.unchecked_ref()).unwrap();
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