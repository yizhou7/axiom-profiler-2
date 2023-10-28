use yew::prelude::*;
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use web_sys::HtmlInputElement;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::JsCast;
use crate::graph_state::*;
use crate::graph::Graph;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let graph_state = use_reducer(GraphState::default);

    let parse_log = {
        let text = props.trace_file_text.to_string();
        let graph_state = graph_state.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let graph_state = graph_state.clone();
            let mut parser = z3parser1::Z3Parser1::new();
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
                    graph_state.dispatch(GUIAction::ParseResult(svg_text, parser.line_nr_of_node));
                },
            );
        })
    };

    let read_input = Callback::from({
        let graph_state = graph_state.clone();
        move |input_event: InputEvent| {
            let target: HtmlInputElement = input_event
                .target()
                .unwrap_throw()
                .dyn_into()
                .unwrap_throw();
            if let Ok(input) = target.value().to_string().parse::<i32>() {
                graph_state.dispatch(GUIAction::ReadInput(input));
            }
        }
    });

    let set_max_line_nr = Callback::from({
        let graph_state = graph_state.clone();
        move |key_event: KeyboardEvent| {
            if key_event.key() == "Enter" {
                let max_line_nr = graph_state.input;
                log::debug!("pressed Enter key");
                graph_state.dispatch(GUIAction::SetMaxLineNr(max_line_nr));
            }
       }
    });

    let uploaded_log = props.trace_file_text.clone();
    use_effect_with(uploaded_log, { 
        let graph_state = graph_state.clone();
        move |_| {
        graph_state.dispatch(GUIAction::ResetState);
    }});

    html! {
        <>
            <div>
                <button onclick={parse_log}>{"Render instantiation graph"}</button>
            </div>
            <div>
                <label for="max_line_nr">{"Render up to line number: "}</label>
                <input type="number" oninput={read_input} onkeypress={set_max_line_nr} id="max_line_nr" />
            </div>
            <ContextProvider<GraphState> context={(*graph_state).clone()}>
                <Graph /> 
            </ContextProvider<GraphState>>
        </>
    }
}

