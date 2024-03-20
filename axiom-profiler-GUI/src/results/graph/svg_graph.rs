use std::sync::{Mutex, OnceLock};

use fxhash::FxHashSet;
use petgraph::graph::{EdgeIndex, NodeIndex};
use smt_log_parser::items::InstIdx;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, SvgsvgElement};
use yew::prelude::*;
use yew::{function_component, html};

use crate::{mouse_position, PrecisePosition};
use crate::results::svg_result::RenderedGraph;

#[derive(Properties, PartialEq, Default)]
pub struct GraphProps {
    pub rendered: Option<RenderedGraph>,
    pub update_selected_nodes: Callback<NodeIndex>,
    pub update_selected_edges: Callback<EdgeIndex>,
    pub zoom_factor: f32,
    pub zoom_factor_delta: f32,
    /// The `NodeIndex` here refers to the original graph!
    pub selected_nodes: Vec<NodeIndex>,
    /// The `EdgeIndex` here refers to the VisibleGraph!
    pub selected_edges: Vec<EdgeIndex>,
    pub scroll_position: PrecisePosition,
    pub set_scroll: Callback<(PrecisePosition, PrecisePosition)>,
    pub scroll_window: NodeRef,
    pub children: Children,
}

#[function_component(Graph)]
pub fn graph(props: &GraphProps) -> Html {
    let Some(rendered) = &props.rendered else {
        return html! {}
    };
    let div_ref = use_node_ref();

    {
        let div_ref = div_ref.clone();
        let zoom_factor = props.zoom_factor as f64;
        let zoom_factor_delta = props.zoom_factor_delta;
        let centered = use_state(|| false);
        let mut scroll_position = props.scroll_position;
        let scroll_window = props.scroll_window.clone();
        let set_scroll = props.set_scroll.clone();
        use_effect_with_deps(
            move |&(_, zoom_factor)| {
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

                    const MARGIN: f64 = 128.0;
                    let (svg_width, svg_height) = (svg_width + 2.0 * MARGIN, svg_height + 2.0 * MARGIN);
                    svg_el.set_attribute("viewBox", format!("{} {} {} {}", -MARGIN, -MARGIN, svg_width, svg_height).as_str()).unwrap();

                    let new_scroll = {
                        let mouse = *mouse_position().read().unwrap();
                        let (x, y) = (mouse.x as f64 - sw_x, mouse.y as f64 - sw_y);
                        // How much of the edge should go to zooming in there
                        const EDGE_ZOOM_BOUNDARY: f64 = 0.15;
                        let (lower_x, upper_x) = (sw_w * EDGE_ZOOM_BOUNDARY, sw_w * (1.0 - EDGE_ZOOM_BOUNDARY));
                        let (lower_y, upper_y) = (sw_h * EDGE_ZOOM_BOUNDARY, sw_h * (1.0 - EDGE_ZOOM_BOUNDARY));
                        let (x, y) = match (x < lower_x, x > upper_x, y < lower_y, y > upper_y) {
                            (true, false, true, false) => (0.0, 0.0),
                            (false, true, true, false) => (sw_w, 0.0),
                            (true, false, false, true) => (0.0, sw_h),
                            (false, true, false, true) => (sw_w, sw_h),
                            _ => (x, y),
                        };

                        if !*centered {
                            // On first render, we want to center the graph
                            centered.set(true);
                            scroll_position = PrecisePosition { x: MARGIN - 10.0, y: MARGIN - 10.0 };
                        }
                        let zoom_factor_delta_chg = 1.0 - (1.0 / zoom_factor_delta as f64);
                        let (width, height) = (x * zoom_factor_delta_chg, y * zoom_factor_delta_chg);
                        let left = (scroll_position.x + width) * zoom_factor_delta as f64;
                        let top = (scroll_position.y + height) * zoom_factor_delta as f64;
                        PrecisePosition { x: left, y: top }
                    };

                    let (scaled_width, scaled_height) = (svg_width * zoom_factor, svg_height * zoom_factor);
                    let _ = svg_el.set_attribute("width", scaled_width.to_string().as_str());
                    let _ = svg_el.set_attribute("height", scaled_height.to_string().as_str());
                    let bounds = PrecisePosition {
                        x: scaled_width - sw_w,
                        y: scaled_height - sw_h,
                    };
                    set_scroll.emit((new_scroll, bounds));
                }
            },
            (rendered.graph.generation, zoom_factor),
        )
    }

    {
        // Whenever selected nodes change, we want to update the visual representation of the nodes
        let div_ref = div_ref.clone();
        let selected_nodes: FxHashSet<_> = props.selected_nodes.iter().copied().collect();

        use_effect_with_deps(
            move |(_, selected_nodes)| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");
                let descendant_nodes = div.get_elements_by_class_name("node");
                for i in 0..descendant_nodes.length() {
                    let node = descendant_nodes.item(i).unwrap();
                    let _ = node.class_list().add_1("clickable");
                    let idx = node.id().strip_prefix("node_").unwrap().parse::<usize>();
                    let idx = NodeIndex::new(idx.unwrap());
                    if selected_nodes.contains(&idx) {
                        let _ = node.class_list().add_1("selected");
                    } else {
                        let _ = node.class_list().remove_1("selected");
                    }
                }
            },
            (rendered.graph.generation, selected_nodes),
        );
    }

    {
        // Whenever selected edges change, we want to update the visual representation of the edges
        let div_ref = div_ref.clone();
        let selected_edges: FxHashSet<_> = props.selected_edges.iter().copied().collect();

        use_effect_with_deps(
            move |(_, selected_edges)| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");
                let descendant_edges = div.get_elements_by_class_name("edge");
                for i in 0..descendant_edges.length() {
                    let edge = descendant_edges.item(i).unwrap();
                    let _ = edge.class_list().add_1("clickable");
                    let idx = edge.id().strip_prefix("edge_").unwrap().parse::<usize>();
                    let idx = EdgeIndex::new(idx.unwrap());
                    if selected_edges.contains(&idx) {
                        let _ = edge.class_list().add_1("selected");
                    } else {
                        let _ = edge.class_list().remove_1("selected");
                    }
                }
            },
            (rendered.graph.generation, selected_edges),
        );
    }

    {
        let nodes_callback = props.update_selected_nodes.clone();
        let edges_callback = props.update_selected_edges.clone();

        let div_ref = div_ref.clone();
        use_effect_with_deps(
            move |_| {
                let div = div_ref
                    .cast::<Element>()
                    .expect("div_ref not attached to div element");

                // construct event_listeners that emit node indices (contained in title tags)
                let descendant_nodes = div.get_elements_by_class_name("node");
                let node_closures: Vec<_> = (0..descendant_nodes.length())
                    .map(|i| {
                        // extract node_index from node to construct callback that emits it
                        let node = descendant_nodes.item(i).unwrap();
                        let idx = node.id().strip_prefix("node_").unwrap().parse::<usize>();
                        let idx = NodeIndex::new(idx.unwrap());
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
                let direct_edges = div.get_elements_by_class_name("edge");
                let edge_closures: Vec<_> = (0..direct_edges.length())
                    .map(|i| {
                        // extract edge_index from edge to construct callback that emits it
                        let edge = direct_edges.item(i).unwrap();
                        let idx = edge.id().strip_prefix("edge_").unwrap().parse::<usize>();
                        let idx = EdgeIndex::new(idx.unwrap());
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
            rendered.graph.generation,
        );
    }
    html! {
        <>
            <div ref={div_ref}>
                {props.children.clone()}
            </div>
        </>
    }
}

#[derive(Properties)]
pub struct SvgProps {
    pub svg: Option<(Html, u32)>,
}
impl PartialEq for SvgProps {
    fn eq(&self, other: &Self) -> bool {
        self.svg.as_ref().map(|(_, i)| *i) == other.svg.as_ref().map(|(_, i)| *i)
    }
}

#[function_component]
pub fn Svg(props: &SvgProps) -> Html {
    props.svg.as_ref().map(|(g, _)| g.clone()).unwrap_or_default()
}
