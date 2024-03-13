use std::sync::{Mutex, OnceLock};

use fxhash::FxHashSet;
use petgraph::graph::EdgeIndex;
use smt_log_parser::items::InstIdx;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, SvgsvgElement};
use yew::prelude::*;
use yew::{function_component, html};

use crate::mouse_position;

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub svg_text: AttrValue,
    pub update_selected_nodes: Callback<usize>,
    pub update_selected_edges: Callback<usize>,
    pub zoom_factor: f32,
    pub zoom_factor_delta: f32,
    pub selected_nodes: Vec<InstIdx>,
    pub selected_edges: Vec<EdgeIndex>,
    pub scroll_window: NodeRef,
}

static ROUNDING_ERROR: OnceLock<Mutex<(f64, f64)>> = OnceLock::new();

#[function_component(Graph)]
pub fn graph(props: &GraphProps) -> Html {
    let svg_result = Html::from_html_unchecked(props.svg_text.clone());
    let div_ref = use_node_ref();

    {
        let div_ref = div_ref.clone();
        let zoom_factor = props.zoom_factor as f64;
        let zoom_factor_delta = props.zoom_factor_delta;
        let scroll_window = props.scroll_window.clone();
        use_effect_with_deps(
            move |_| {
                gloo_timers::callback::Timeout::new(100, move || {
                    let scroll_window = scroll_window.cast::<Element>().unwrap();
                    let rect = scroll_window.get_bounding_client_rect();
                    scroll_window.scroll_to_with_x_and_y(rect.width() / 2.0 - 10.0, rect.height() / 2.0 - 10.0);
                }).forget();
            },
            (),
        );
        let svg_text = props.svg_text.clone();
        let scroll_window = props.scroll_window.clone();
        use_effect_with_deps(
            move |_| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");
                // setting the transform-origin to the top left corner of the surrounding div
                // we know there is only a single SVG element, hence can just take item(0)
                let svg_element = div.get_elements_by_tag_name("svg").item(0);
                if let Some(el) = svg_element {
                    let svg_el = el.dyn_into::<SvgsvgElement>().ok().unwrap();
                    let view_box: Vec<f64> = svg_el.get_attribute("viewBox").unwrap().split_ascii_whitespace().map(|s| s.parse().unwrap()).collect();
                    let (svg_width, svg_height) = (view_box[2] + 2.0 * view_box[0], view_box[3] + 2.0 * view_box[1]);

                    let scroll_window = scroll_window.cast::<Element>().unwrap();
                    let rect = scroll_window.get_bounding_client_rect();
                    let (sw_x, sw_y, sw_w, sw_h) = (rect.x(), rect.y(), rect.width(), rect.height());

                    let (svg_width, svg_height) = (svg_width + sw_w, svg_height + sw_h);
                    svg_el.set_attribute("viewBox", format!("{} {} {} {}", -sw_w / 2.0, -sw_h / 2.0, svg_width, svg_height).as_str()).unwrap();

                    let new_scroll = {
                        let mouse = *mouse_position().read().unwrap();
                        let (x, y) = (mouse.x as f64 - sw_x, mouse.y as f64 - sw_y);
                        // How much of the edge should go to zooming in there
                        const EDGE_ZOOM_BOUNDARY: f64 = 0.1;
                        let (lower_x, upper_x) = (sw_w * EDGE_ZOOM_BOUNDARY, sw_w * (1.0 - EDGE_ZOOM_BOUNDARY));
                        let x = if x < lower_x {
                            0.0
                        } else if x > upper_x {
                            sw_w
                        } else {
                            x
                        };
                        let (lower_y, upper_y) = (sw_h * EDGE_ZOOM_BOUNDARY, sw_h * (1.0 - EDGE_ZOOM_BOUNDARY));
                        let y = if y < lower_y {
                            0.0
                        } else if y > upper_y {
                            sw_h
                        } else {
                            y
                        };

                        let zoom_factor_delta_chg = 1.0 - (1.0 / zoom_factor_delta as f64);
                        let (width, height) = (x * zoom_factor_delta_chg, y * zoom_factor_delta_chg);

                        let rounding_error = ROUNDING_ERROR.get_or_init(|| Mutex::new((0.0, 0.0)));
                        let mut rounding_error = rounding_error.lock().unwrap();
                        let left = (scroll_window.scroll_left() as f64 + rounding_error.0 + width) * zoom_factor_delta as f64;
                        let new_left = left.round();
                        rounding_error.0 = left - new_left;
                        let top = (scroll_window.scroll_top() as f64 + rounding_error.1 + height) * zoom_factor_delta as f64;
                        let new_top = top.round();
                        rounding_error.1 = top - new_top;
                        (new_left as i32, new_top as i32)
                    };

                    let (scaled_width, scaled_height) = (svg_width * zoom_factor, svg_height * zoom_factor);
                    let _ = svg_el.set_attribute("width", scaled_width.to_string().as_str());
                    let _ = svg_el.set_attribute("height", scaled_height.to_string().as_str());
                    // let _ = svg_el.set_attribute("transform-origin", "top left");
                    web_sys::console::log_1(&"Updating the transform-attribute of svg to ".into());
                    web_sys::console::log_1(&zoom_factor.into());
                    // let _ = svg_el.set_attribute(
                    //     "style",
                    //     format!("transform: scale({});", zoom_factor).as_str(),
                    // );
                    scroll_window.set_scroll_left(new_scroll.0 as i32);
                    scroll_window.set_scroll_top(new_scroll.1 as i32);
                }
            },
            (svg_text, zoom_factor),
        )
    }

    {
        // Whenever selected nodes change, we want to update the visual representation of the nodes
        let div_ref = div_ref.clone();
        let selected_nodes: FxHashSet<_> = props.selected_nodes.iter().copied().map(usize::from).collect();
        let selected_nodes_ref = selected_nodes.clone();

        use_effect_with_deps(
            move |_| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");
                let descendant_nodes = div.get_elements_by_class_name("node");
                for i in 0..descendant_nodes.length() {
                    let node = descendant_nodes.item(i).unwrap();
                    let _ = node.class_list().add_1("clickable");
                    let idx = node.id().strip_prefix("node").unwrap().parse::<usize>().unwrap();
                    if selected_nodes.contains(&idx) {
                        let _ = node.class_list().add_1("selected");
                    } else {
                        let _ = node.class_list().remove_1("selected");
                    }
                }
            },
            selected_nodes_ref,
        );
    }

    {
        // Whenever selected edges change, we want to update the visual representation of the edges
        let div_ref = div_ref.clone();
        let selected_edges: FxHashSet<_> = props.selected_edges.iter().copied().map(EdgeIndex::index).collect();
        let selected_edges_ref = selected_edges.clone();

        use_effect_with_deps(
            move |_| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");
                let descendant_edges = div.get_elements_by_class_name("edge direct");
                for i in 0..descendant_edges.length() {
                    let edge = descendant_edges.item(i).unwrap();
                    let _ = edge.class_list().add_1("clickable");
                    let idx = edge.id().strip_prefix("edge").unwrap().parse::<usize>().unwrap();
                    if selected_edges.contains(&idx) {
                        let _ = edge.class_list().add_1("selected");
                    } else {
                        let _ = edge.class_list().remove_1("selected");
                    }
                }
            },
            selected_edges_ref,
        );
    }

    {
        let svg_text = props.svg_text.clone();
        let nodes_callback = props.update_selected_nodes.clone();
        let edges_callback = props.update_selected_edges.clone();

        let div_ref = div_ref.clone();
        use_effect_with_deps(
            move |_| {
                web_sys::console::log_1(&"Using effect".into());
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");

                // construct event_listeners that emit node indices (contained in title tags)
                let descendant_nodes = div.get_elements_by_class_name("node");
                let node_closures: Vec<_> = (0..descendant_nodes.length())
                    .map(|i| {
                        // extract node_index from node to construct callback that emits it
                        let node = descendant_nodes.item(i).unwrap();
                        let idx = node.id().strip_prefix("node").unwrap().parse::<usize>().unwrap();
                        let callback = nodes_callback.clone();
                        let closure: Closure<dyn Fn(Event)> = Closure::new(move |_| callback.emit(idx));
                        // attach event listener to node
                        node.add_event_listener_with_callback(
                            "click",
                            closure.as_ref().unchecked_ref(),
                        ).unwrap();
                        let cancel: Closure<dyn Fn(Event)> = Closure::new(move |e: Event| {
                            e.cancel_bubble(); e.stop_propagation();
                        });
                        node.add_event_listener_with_callback(
                            "mousedown",
                            cancel.as_ref().unchecked_ref(),
                        ).unwrap();
                        (cancel, closure)
                    })
                    .collect();
                let direct_edges = div.get_elements_by_class_name("edge direct");
                let edge_closures: Vec<_> = (0..direct_edges.length())
                    .map(|i| {
                        // extract edge_index from edge to construct callback that emits it
                        let edge = direct_edges.item(i).unwrap();
                        let idx = edge.id().strip_prefix("edge").unwrap().parse::<usize>().unwrap();
                        let callback = edges_callback.clone();
                        let closure: Closure<dyn Fn(Event)> = Closure::new(move |_| callback.emit(idx));
                        // attach event listener to edge
                        edge.add_event_listener_with_callback(
                            "click",
                            closure.as_ref().unchecked_ref(),
                        ).unwrap();
                        let cancel: Closure<dyn Fn(Event)> = Closure::new(move |e: Event| {
                            e.cancel_bubble(); e.stop_propagation();
                        });
                        edge.add_event_listener_with_callback(
                            "mousedown",
                            cancel.as_ref().unchecked_ref(),
                        ).unwrap();
                        (cancel, closure)
                    })
                    .collect();
                move || {
                    for i in 0..node_closures.len() {
                        if let Some(node) = descendant_nodes.item(i as u32) {
                            let (cancel, closure) = &node_closures[i];
                            node.remove_event_listener_with_callback(
                                "click",
                                closure.as_ref().unchecked_ref(),
                            ).unwrap();
                            node.remove_event_listener_with_callback(
                                "mousedown",
                                cancel.as_ref().unchecked_ref(),
                            ).unwrap();
                        }
                    }
                    for i in 0..edge_closures.len() {
                        if let Some(edge) = direct_edges.item(i as u32) {
                            let (cancel, closure) = &edge_closures[i];
                            edge.remove_event_listener_with_callback(
                                "click",
                                closure.as_ref().unchecked_ref(),
                            ).unwrap();
                            edge.remove_event_listener_with_callback(
                                "mousedown",
                                cancel.as_ref().unchecked_ref(),
                            ).unwrap();
                        }
                    }
                }
            },
            svg_text,
        );
    }
    html! {
        <>
            <div id="testing-id" ref={div_ref}>
                {svg_result}
            </div>
        </>
    }
}
