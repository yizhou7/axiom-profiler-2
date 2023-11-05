use std::time::Duration;

use smt_log_parser::parsers::z3::results::InstGraph;
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use yew::prelude::*;
use smt_log_parser::parsers::LogParser;
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::graph::{Graph, GraphProps};
use crate::input_state::{UsizeInput, InputValue};

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let graph_props = use_state(|| GraphProps::default());
    let max_log_line_nr = use_reducer(InputValue::default);
    let max_instantiations = use_reducer(InputValue::default);

    let parse_log = {
        let graph_props = graph_props.clone();
        let trace_file_text = props.trace_file_text.clone();
        // let max_log_line_nr = max_log_line_nr.value.clone();
        // let max_instantiations = max_instantiations.value.clone();
        Callback::from(move |_| {
            let graph_props = graph_props.clone();
            let trace_file_text = trace_file_text.clone();
            let parser = Z3Parser::from_str(trace_file_text.as_str());
            // let parser = parser.process_all();
            let (result, parser) = parser.process_all_timeout(Duration::new(0, 100_000_000));
            match result {
                None => log::debug!("processed entire log"),
                _ => log::debug!("timeout reached before processing entire log"),
            }
            // let (qi_graph, line_nr_of_node) = parser.get_instantiation_graph();
            let InstGraph{inst_graph, line_nr_of_node, ..} = parser.get_instantiation_graph();
            let dot_output = format!("{:?}", Dot::with_config(&inst_graph, &[Config::EdgeNoLabel])); 
            wasm_bindgen_futures::spawn_local(
                async move {
                   let graphviz = VizInstance::new().await;
                //    log::debug!("Available engines in GraphViz:");
                //    for format in graphviz.engines() {
                //     log::debug!("{format}");
                //    } 
                   let mut options = viz_js::Options::default();
                //    options.engine = "fdp".to_string();
                    let svg = graphviz
                        .render_svg_element(dot_output, options)
                        .expect("Could not render graphviz");
                    let svg_text = svg.outer_html();
                    graph_props.set(GraphProps{svg_text: AttrValue::from(svg_text), line_nr_of_node});
                },
            );
        })
    };

    // this resets the graph-props whenever a new log-file has been uploaded
    let uploaded_log = props.trace_file_text.clone();
    use_effect_with(uploaded_log, { 
        let graph_props = graph_props.clone();
        move |_| {
        graph_props.set(GraphProps::default());
    }});

    html! {
        <>
            <div>
                <UsizeInput label={"Parse log up to which line number? "} dependency={props.trace_file_text.clone()} input_value={max_log_line_nr} />
                <UsizeInput label={"Parse log up to how many instantiations? "} dependency={props.trace_file_text.clone()} input_value={max_instantiations} />
                <button onclick={parse_log}>{"Parse log and render results"}</button>
            </div>
            <Graph svg_text={graph_props.svg_text.clone()} line_nr_of_node={graph_props.line_nr_of_node.clone()} /> 
        </>
    }
}

