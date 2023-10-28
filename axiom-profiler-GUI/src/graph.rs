use yew::prelude::*;
use crate::svg_result::GraphState;
use wasm_bindgen::{prelude::Closure, JsValue};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};

#[function_component(Graph)]
pub fn graph() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    log::debug!("Cloning SVG!");
    let svg_attr_val = AttrValue::from((*state.svg_text).clone());
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
    {
        // Executes whenever max_line_nr is updated
        let div_ref = div_ref.clone();
        let max_line_nr = max_line_nr.clone();

        use_effect_with(max_line_nr, move |&max_line_nr| {
            let max_line_nr_js = JsValue::from(max_line_nr);
            let div = div_ref
                .cast::<HtmlElement>()
                .expect("div_ref not attached to div element");

            // Iterate through all nodes with class "node" set visibility depending on line_nr 
            let descendant_nodes = div.get_elements_by_class_name("node");
            for i in 0..descendant_nodes.length() {
                if let Some(node) = descendant_nodes.item(i) {
                    let text_element = node.query_selector("text").expect("Failed to select text element");
                    if let Some(text) = text_element {
                        if let Some(text_content) = text.text_content() {
                            match text_content.parse::<i32>() {
                                Ok(line_nr) => if line_nr > max_line_nr {
                                    node.set_attribute("visibility", "hidden");
                                } else {
                                    node.set_attribute("visibility", "visible");
                                },
                                Err(_) => { 
                                    node.set_attribute("visibility", "visible"); }
                            }
                        }
                        
                    }
                }
            }
            // Iterate through all elements with class "edge" set visibility depending on line_nr 
            let descendant_edges = div.get_elements_by_class_name("edge");
            for i in 0..descendant_edges.length() {
                if let Some(edge) = descendant_edges.item(i) {
                    let title_element = edge.query_selector("title").expect("Failed to select title element");
                    if let Some(title) = title_element {
                        if let Some(title_content) = title.text_content() {
                            match parse_edge_title(&title_content) {
                                Some((from_node_idx, to_node_idx)) => {
                                    match (state.line_nr_of_node.get(&from_node_idx), state.line_nr_of_node.get(&to_node_idx)) {
                                        (Some(&from_node_line_nr), Some(&to_node_line_nr)) => {
                                            if from_node_line_nr > state.max_line_nr as usize || to_node_line_nr > state.max_line_nr as usize {
                                                edge.set_attribute("visibility", "hidden");
                                            } else {
                                                edge.set_attribute("visibility", "visible");
                                            }
                                        },
                                        (_, _) => {edge.set_attribute("visibility", "visible");}
                                    }
                                }, 
                                _ => {edge.set_attribute("visibility", "visible");}

                            }
                        }
                    }

                }
            }
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

fn parse_edge_title(input: &str) -> Option<(usize, usize)> {
    // Split the input string by "->" and parse the integers
    let parts: Vec<&str> = input.trim().split("->").collect();

    if parts.len() == 2 {
        let x = parts[0].trim().parse::<i32>();
        let y = parts[1].trim().parse::<i32>();

        // Check if parsing was successful
        match (x, y) {
            (Ok(x_val), Ok(y_val)) => Some((x_val as usize, y_val as usize)),
            _ => None,
        }
    } else {
        None
    }
}