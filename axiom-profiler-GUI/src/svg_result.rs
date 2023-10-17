use std::collections::BTreeMap;
use yew::prelude::*;
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use web_sys::HtmlInputElement;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::JsCast;

pub enum GUIAction {
    ParseResult(bool, String, BTreeMap<usize, usize>),
    SetMaxLineNr(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphState {
    pub parsed_log: bool,
    pub svg_text: String,
    pub line_nr_of_node: BTreeMap<usize, usize>,
    pub max_line_nr: i32,
}

impl Default for GraphState {
    fn default() -> Self {
        Self { 
            parsed_log: false, 
            svg_text: String::new(), 
            line_nr_of_node: BTreeMap::new(), 
            max_line_nr: i32::MAX}
    }
}

impl Reducible for GraphState {
    type Action = GUIAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            GUIAction::ParseResult(parsed_log, svg_text, line_nr_of_node) => Self {
                parsed_log,
                svg_text, 
                line_nr_of_node, 
                max_line_nr: self.max_line_nr,
            }.into(),
            GUIAction::SetMaxLineNr(max_line_nr) => Self {
                parsed_log: self.parsed_log,
                svg_text: self.svg_text.clone(),
                line_nr_of_node: self.line_nr_of_node.clone(),
                max_line_nr,
            }.into(),
        }
    }
}

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
                    graph_state.dispatch(GUIAction::ParseResult(true, svg_text, parser.line_nr_of_node));
                },
            );
        })
    };

    let set_max_line_nr = Callback::from({
        let graph_state = graph_state.clone();
        move |input_event: InputEvent| {
            let target: HtmlInputElement = input_event
                .target()
                .unwrap_throw()
                .dyn_into()
                .unwrap_throw();
            if let Ok(max_line_nr) = target.value().to_string().parse::<i32>() {
                graph_state.dispatch(GUIAction::SetMaxLineNr(max_line_nr));
            }
        }
    });

    use crate::graph::Graph;
    html! {
        <>
            <div>
                <button onclick={parse_log}>{"Render instantiation graph"}</button>
            </div>
            <div>
                <label for="max_line_nr">{"Render up to line number: "}</label>
                <input type="number" oninput={set_max_line_nr} id="max_line_nr" />
            </div>
            { if graph_state.parsed_log {
                html! {
                    <ContextProvider<GraphState> context={(*graph_state).clone()}>
                        <Graph /> 
                    </ContextProvider<GraphState>>
                }
            } else {
                html! {}
            }}
        </>
    }
}

