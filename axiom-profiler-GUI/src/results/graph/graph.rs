use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement, SvgsvgElement};
use yew::prelude::*;
use yew::{function_component, html, use_node_ref, Html};

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub svg_text: AttrValue,
    pub update_selected_node: Callback<usize>,
    pub zoom_factor: f32,
}

#[function_component(Graph)]
pub fn graph(props: &GraphProps) -> Html {
    let svg_result = Html::from_html_unchecked(props.svg_text.clone());
    let div_ref = use_node_ref();

    {
        let div_ref = div_ref.clone();
        let zoom_factor = props.zoom_factor;
        let svg_text = props.svg_text.clone();
        use_effect_with_deps(
            move |_| {
                let div = div_ref
                    .cast::<HtmlElement>()
                    .expect("div_ref not attached to div element");
                // setting the transform-origin to the top left corner of the surrounding div
                // we know there is only a single SVG element, hence can just take item(0)
                let svg_element = div.get_elements_by_tag_name("svg").item(0);
                if let Some(el) = svg_element {
                    let svg_el = el.dyn_into::<SvgsvgElement>().ok().unwrap();
                    let _ = svg_el.set_attribute("transform-origin", "top left");
                    web_sys::console::log_1(&"Updating the transform-attribute of svg to ".into());
                    web_sys::console::log_1(&zoom_factor.into());
                    let _ = svg_el.set_attribute(
                        "style",
                        format!("transform: scale({});", zoom_factor).as_str(),
                    );
                }
            },
            (svg_text, zoom_factor),
        )
    }

    {
        // Whenever SVG text changes, need to attach event listeners to new nodes
        let div_ref = div_ref.clone();
        let svg_text = props.svg_text.clone();
        let callback = props.update_selected_node.clone();

        use_effect_with_deps(
            move |_| {
                web_sys::console::log_1(&"Using effect".into());
                let div = div_ref
                    .cast::<HtmlElement>()
                    .expect("div_ref not attached to div element");

                // construct event_listeners that emit node indices (contained in title tags)
                let descendant_nodes = div.get_elements_by_class_name("node");
                let closures: Vec<Closure<dyn Fn(Event)>> = (0..descendant_nodes.length())
                    .map(|i| {
                        // extract node_index from node to construct callback that emits it
                        let node = descendant_nodes.item(i).unwrap();
                        let text_element = node
                            .query_selector("text")
                            .expect("Failed to select title element")
                            .unwrap();
                        let text_content = text_element.text_content().unwrap();
                        let ellipse = node
                            .query_selector("ellipse")
                            .expect("Failed to select title element")
                            .unwrap();
                        let node_index = text_content.parse::<usize>().unwrap();
                        let callback = callback.clone();
                        let closure: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
                            // the selected node should become bold whenever it's clicked on the first time
                            // after that it should also toggle between bold and normal when the user repeatedly
                            // clicks on the node
                            let current_stroke_width = ellipse.get_attribute("stroke-width");
                            match current_stroke_width {
                                None => {
                                    let _ = ellipse.set_attribute("stroke-width", "3");
                                }
                                Some(ref width) => match width.parse::<usize>() {
                                    Ok(1) => {
                                        let _ = ellipse.set_attribute("stroke-width", "3");
                                    }
                                    Ok(3) => {
                                        let _ = ellipse.set_attribute("stroke-width", "1");
                                    }
                                    _ => (),
                                },
                            };
                            callback.emit(node_index);
                        });
                        // attach event listener to node
                        node.add_event_listener_with_callback(
                            "click",
                            closure.as_ref().unchecked_ref(),
                        )
                        .unwrap();
                        closure
                    })
                    .collect();

                move || {
                    // Remove event listeners when the component is unmounted
                    for i in 0..closures.len() {
                        if let Some(node) = descendant_nodes.item(i as u32) {
                            let closure = closures.as_slice()[i as usize].as_ref();
                            node.remove_event_listener_with_callback(
                                "click",
                                closure.unchecked_ref(),
                            )
                            .unwrap();
                        }
                    }
                }
            },
            svg_text,
        );
    }
    html! {
        <div ref={div_ref}>
            {svg_result}
        </div>
    }
}
