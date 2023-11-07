use smt_log_parser::parsers::z3::results::{InstGraph, RenderSettings};
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use yew::prelude::*;
use yew_hooks::prelude::*;
use smt_log_parser::parsers::LogParser;
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::graph::{Graph, GraphProps};
use crate::input_state::{UsizeInput, InputValue};
use crate::toggle_switch::ToggleSwitch;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let graph_props = use_state(|| GraphProps::default());
    let max_line_nr = use_reducer(InputValue::default);
    let exclude_theory_inst = use_bool_toggle(true);
    let max_instantiations = use_reducer(InputValue::default);

    let parse_log = {
        let graph_props = graph_props.clone();
        let trace_file_text = props.trace_file_text.clone();
        let max_line_nr = max_line_nr.value.clone();
        let exclude_theory_inst = *exclude_theory_inst.clone();
        let max_instantiations = max_instantiations.value.clone();
        Callback::from(move |_| {
            let graph_props = graph_props.clone();
            let trace_file_text = trace_file_text.clone();
            let parser = Z3Parser::from_str(trace_file_text.as_str());
            let parser = parser.process_all();
            let InstGraph{inst_graph, line_nr_of_node, ..} = parser.get_instantiation_graph(RenderSettings{
                max_line_nr,
                exclude_theory_inst,
                max_instantiations,
            });
            log::debug!("Finished building QI graph");
            let dot_output = format!("{:?}", Dot::with_config(&inst_graph, &[Config::EdgeNoLabel])); 
            log::debug!("Finished building dot output");
            wasm_bindgen_futures::spawn_local(
                async move {
                   let graphviz = VizInstance::new().await;
                   let mut options = viz_js::Options::default();
                //    options.engine = "osage".to_string();
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
                <h2>{"Specify (optional) render settings"}</h2>
                <UsizeInput label={"Render graph up to line number "} dependency={props.trace_file_text.clone()} input_value={max_line_nr} default_value={usize::MAX} />
                <ToggleSwitch label={"Ignore theory-solving instantiations: "} dependency={props.trace_file_text.clone()} input_value={exclude_theory_inst} />
                <UsizeInput label={"Render the n most expensive instantiations where n = "} dependency={props.trace_file_text.clone()} input_value={max_instantiations} default_value={250}/>
                <button onclick={parse_log}>{"Render graph"}</button>
            </div>
            <Graph svg_text={graph_props.svg_text.clone()} line_nr_of_node={graph_props.line_nr_of_node.clone()} /> 
        </>
    }
}

