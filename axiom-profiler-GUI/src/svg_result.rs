use std::collections::BTreeMap;
use yew::prelude::*;
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};

#[derive(Clone, Debug, PartialEq)]
pub struct GraphCtxt {
    pub max_line_nr: i32,
    pub line_nr_of_node: BTreeMap<usize, usize>,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| None);
    let onclick = {
        let text = props.trace_file_text.to_string();
        let svg_text = svg_text.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let svg_text = svg_text.clone();
            log::debug!("use effect");
            wasm_bindgen_futures::spawn_local(
                async move {
                    let mut parser = z3parser1::Z3Parser1::new();
                    parser.process_log(text);
                    let qi_graph = parser.get_instantiation_graph();
                    let dot_output = format!("{:?}", Dot::with_config(qi_graph, &[Config::EdgeNoLabel])); 
                    let graphviz = VizInstance::new().await;
                    let svg = graphviz
                        .render_svg_element(dot_output, viz_js::Options::default())
                        .expect("Could not render graphviz");
                    let fetched_svg = svg.outer_html(); 
                    svg_text.set(Some((fetched_svg, parser.line_nr_of_node)));
                },
                   
            );
        })
    };
    use crate::graph::Graph;
    html! {
        <>
            <div>
                <button onclick={onclick}>{"Render instantiation graph"}</button>
            </div>
            { if let Some((text, line_nr_of_node)) = (*svg_text).clone() {
                html! {
                    <ContextProvider<GraphCtxt> context={GraphCtxt{ max_line_nr: i32::MAX, line_nr_of_node } }>
                        <Graph svg_text={text} /> 
                    </ContextProvider<GraphCtxt>>
                }
            } else {
                html! {}
            }}
        </>
    }
}

