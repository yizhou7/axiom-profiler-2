use yew::prelude::*;
use crate::graph_state::*;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};
use yew::{function_component, html, use_effect_with, use_node_ref, Html};
use web_sys::HtmlInputElement;
use wasm_bindgen::UnwrapThrowExt;
use std::collections::BTreeMap;

#[derive(Properties, PartialEq)]
pub struct GraphProps {
    pub svg_text: String,
    pub line_nr_of_node: BTreeMap<usize, usize>,
}

#[function_component(Graph)]
pub fn graph(props: &GraphProps) -> Html {
    let graph_state = use_reducer(GraphState::default);
    let svg_attr_val = AttrValue::from(props.svg_text.clone());
    let svg_result = Html::from_html_unchecked(svg_attr_val);
    let div_ref = use_node_ref();
    let input_ref = use_node_ref();
    {
        // Whenever SVG text changes, need to add new listener to new nodes 
        let div_ref = div_ref.clone();
        let svg_text = props.svg_text.clone();

        use_effect_with(svg_text, move |svg_text| {
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
        // Whenever max_line_nr or svg_text is updated, need to conditionally render the nodes and edges
        let div_ref = div_ref.clone();
        let max_line_nr = graph_state.max_line_nr.clone();
        let svg_text = props.svg_text.clone();
        let line_nr_of_node = props.line_nr_of_node.clone();
        let graph_state = graph_state.clone();

        use_effect_with((max_line_nr,svg_text), move |&(max_line_nr, _)| {
            web_sys::console::log_1(&"Using effect due to max_line_nr change".into());
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
                            let visibility = match parse_edge_title(&title_content) {
                                Some((from_node_idx, to_node_idx)) => {
                                    match (line_nr_of_node.get(&from_node_idx), line_nr_of_node.get(&to_node_idx)) {
                                        (Some(&from_node_line_nr), Some(&to_node_line_nr)) => {
                                            if from_node_line_nr > graph_state.max_line_nr as usize || to_node_line_nr > graph_state.max_line_nr as usize {
                                                "hidden"
                                            } else {
                                                "visible"
                                            }
                                        },
                                        (_, _) => "visible"
                                    }
                                }, 
                                _ =>  "visible"
                            };
                            edge.set_attribute("visibility", visibility);
                        }
                    }
                }
            }
            move || {
                web_sys::console::log_1(&"Tearing down".into());
            }
        });
    }
    {
        let svg_text = props.svg_text.clone();
        let line_nr_of_node = props.line_nr_of_node.clone();
        let graph_state = graph_state.clone();
        let input_ref = input_ref.clone();
        use_effect_with((svg_text, line_nr_of_node), {
            let graph_state = graph_state.clone();
            move |_| {
                graph_state.dispatch(GUIAction::ResetState);
                let input = input_ref
                    .cast::<HtmlInputElement>()
                    .expect("div_ref not attached to div element");
                input.set_value("");
            }
        });
    }

    let read_input = Callback::from({
        let graph_state = graph_state.clone();
        move |input_event: InputEvent| {
            let target: HtmlInputElement = input_event
                .target()
                .unwrap_throw()
                .dyn_into()
                .unwrap_throw();
            if let Ok(input) = target.value().to_string().parse::<i32>() {
                graph_state.dispatch(GUIAction::ReadInput(input));
            } else {
                // by default, i.e., if user input can't be parsed as i32, we
                // reset the state of the graph
                graph_state.dispatch(GUIAction::ResetState);
            }
        }
    });

    let set_max_line_nr = Callback::from({
        let graph_state = graph_state.clone();
        move |key_event: KeyboardEvent| {
            if key_event.key() == "Enter" {
                let max_line_nr = graph_state.input;
                graph_state.dispatch(GUIAction::SetMaxLineNr(max_line_nr));
            }
       }
    });

    html! {
        <>
            <div>
                <label for="max_line_nr">{"Render up to line number: "}</label>
                <input ref={input_ref} type="number" oninput={read_input} onkeypress={set_max_line_nr} id="max_line_nr" />
            </div>
            <div ref={div_ref}>
                {svg_result}
            </div>
        </>
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