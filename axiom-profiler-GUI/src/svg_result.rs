use yew::prelude::*;
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
// use web_sys::HtmlInputElement;
// use wasm_bindgen::UnwrapThrowExt;
// use wasm_bindgen::JsCast;
// use crate::graph_state::*;
use crate::graph::Graph;
use std::collections::BTreeMap;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    // let graph_state = use_reducer(GraphState::default);
    let state = use_state(|| (String::new(), BTreeMap::new()));

    let parse_log = {
        let text = props.trace_file_text.to_string();
        let state = state.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let state = state.clone();
            let mut parser = z3parser1::Z3Parser1::new();
            // parser.process_log(text);
            parser.process_log(text);
            let qi_graph = parser.get_instantiation_graph();
            let dot_output = format!("{:?}", Dot::with_config(qi_graph, &[Config::EdgeNoLabel])); 
            log::debug!("use effect");
            wasm_bindgen_futures::spawn_local(
                async move {
                   let graphviz = VizInstance::new().await;
                    let svg = graphviz
                        .render_svg_element(dot_output, viz_js::Options::default())
                        .expect("Could not render graphviz");
                    let svg_text = svg.outer_html();
                    state.set((svg_text, parser.line_nr_of_node))
                },
            );
        })
    };

    let uploaded_log = props.trace_file_text.clone();
    use_effect_with(uploaded_log, { 
        let state = state.clone();
        move |_| {
        state.set((String::new(), BTreeMap::new()));
    }});

    html! {
        <>
            <div>
                <button onclick={parse_log}>{"Render instantiation graph"}</button>
            </div>
            <Graph svg_text={state.0.clone()} line_nr_of_node={state.1.clone()} /> 
        </>
    }
}

