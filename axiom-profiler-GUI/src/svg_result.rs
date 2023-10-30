use std::time::Duration;

use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use yew::prelude::*;
use smt_log_parser::parsers::{z3::z3parser, LogParser, StreamParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::graph::{Graph, GraphProps};
use crate::input_state::{IntegerInput, InputValue};

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
        let max_log_line_nr = max_log_line_nr.value.clone();
        let max_instantiations = max_instantiations.value.clone();
        Callback::from(move |_| {
            let graph_props = graph_props.clone();
            let trace_file_text = trace_file_text.clone();
            // z3parser::new_with_settings(ParserSettings{
            //     max_line_nr: max_log_line_nr as u32, 
            //     max_instantiations: max_instantiations as u32, 
            // });
            let (_, mut parser): (_, Z3Parser) = StreamParser::parse_entire_string(trace_file_text.as_str(), Duration::ZERO);
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
                    graph_props.set(GraphProps{svg_text: AttrValue::from(svg_text), line_nr_of_node: parser.line_nr_of_node});
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
                <IntegerInput label={"Parse log up to which line number? "} dependency={props.trace_file_text.clone()} input_value={max_log_line_nr} />
                <IntegerInput label={"Parse log up to how many instantiations? "} dependency={props.trace_file_text.clone()} input_value={max_instantiations} />
                <button onclick={parse_log}>{"Parse log and render results"}</button>
            </div>
            <Graph svg_text={graph_props.svg_text.clone()} line_nr_of_node={graph_props.line_nr_of_node.clone()} /> 
        </>
    }
}

