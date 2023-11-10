use smt_log_parser::parsers::z3::results::FilterSettings;
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use yew::prelude::*;
use smt_log_parser::parsers::LogParser;
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::graph::{Graph, GraphProps};
use crate::graph_filter::*;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let filter_params = use_state(|| FilterSettings::default());
    let graph_props = use_state(|| GraphProps::default());
    let parser = use_state(|| Z3Parser::from_str(&props.trace_file_text.as_str()).process_all());
    let inst_graph = use_state(|| parser.compute_instantiation_graph());

    {
        let graph_props = graph_props.clone();
        let filter_params = *filter_params;
        let inst_graph = &mut (*inst_graph);
        use_effect_with(filter_params, move |_| {
            inst_graph.filter(filter_params);
            log::debug!("Finished building QI graph");
            let graph = &inst_graph.filtered_inst_graph;
            let dot_output = format!("{:?}", Dot::with_config(graph, &[Config::EdgeNoLabel])); 
            log::debug!("Finished building dot output");
            wasm_bindgen_futures::spawn_local(
                async move {
                    let graphviz = VizInstance::new().await;
                    let mut options = viz_js::Options::default();
                    options.engine = "circo".to_string();
                    let svg = graphviz
                        .render_svg_element(dot_output, options)
                        .expect("Could not render graphviz");
                    let svg_text = svg.outer_html();
                    graph_props.set(GraphProps{svg_text: AttrValue::from(svg_text)});
                },
            );
        });
    }

    // this resets the graph-props whenever a new log-file has been uploaded
    let uploaded_log = props.trace_file_text.clone();
    use_effect_with(uploaded_log, { 
        let graph_props = graph_props.clone();
        move |_| {
        graph_props.set(GraphProps::default());
    }});

    html! {
        <>  
            <GraphFilter title={"Specify (optional) render settings:"} filter_settings={filter_params} dependency={props.trace_file_text.clone() }/>
            <Graph svg_text={graph_props.svg_text.clone()} /> 
        </>
    }
}

