use wasm_bindgen::prelude::*;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Object, Reflect};
use petgraph::Graph;

#[wasm_bindgen(module = "/js/graph_layout.js")]
extern "C" {
    fn get_node_positions(dagDataJson: JsValue) -> Object;
}

struct Node {
    data: String,
    x: usize,
    y: usize,
}

struct Edge {
    data: String,
    points: ((usize, usize), (usize, usize)),
    source: (usize, usize), 
}

#[derive(Default)]
pub struct GraphLayout {
    edges: Vec<Node>,
    nodes: Vec<Edge>,
    pub svg_width: f64,
    pub svg_height: f64,
}

pub fn get_sugiyama_layout(graph: &Graph<usize,()>) -> GraphLayout {
    let serialized_graph = serde_json::to_string(&graph).unwrap();
    let js_serialized_graph = JsValue::from_serde(&serialized_graph).unwrap();
    let node_positions_obj = get_node_positions(js_serialized_graph);
    // Access the 'width' and 'height' fields
    let width: f64 = Reflect::get(&node_positions_obj, &"width".into())
        .unwrap()
        .as_f64()
        .unwrap();
    let height: f64 = Reflect::get(&node_positions_obj, &"height".into())
        .unwrap()
        .as_f64()
        .unwrap();

    let mut layout = GraphLayout::default();
    layout.svg_height = height;
    layout.svg_width = width;
    layout
}