use yew::prelude::*;
use smt_log_parser::parsers::{z3::{results::{FilterSettings, InstGraph}, z3parser::Z3Parser}, LogParser};
use crate::graph_filter::GraphFilter;
use crate::graph::Graph;
use petgraph::dot::{Dot, Config};
use viz_js::VizInstance;

pub enum Msg {
    RecomputeSvg(FilterSettings),
    UpdateSvgText(AttrValue),
}

pub struct SVGResult {
    pub svg_text: AttrValue,
    pub inst_graph: InstGraph,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(ctx: &Context<Self>) -> Self {
        let parser = Z3Parser::from_str(ctx.props().trace_file_text.as_str()).process_all();
        Self {
            svg_text: AttrValue::default(),
            inst_graph: parser.compute_instantiation_graph(), 
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RecomputeSvg(settings) => {
                let filtered_graph = self.inst_graph.filter(settings);
                let dot_output = format!("{:?}", Dot::with_config(filtered_graph, &[Config::EdgeNoLabel])); 
                log::debug!("Finished building dot output");
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let graphviz = VizInstance::new().await;
                    let options = viz_js::Options::default();
                    // options.engine = "circo".to_string();
                    let svg = graphviz
                        .render_svg_element(dot_output, options)
                        .expect("Could not render graphviz");
                    let svg_text = svg.outer_html();
                    link.send_message(Msg::UpdateSvgText(AttrValue::from(svg_text)));
                    },
                );
                // only need to re-render once the new SVG has been set
                false 
            },
            Msg::UpdateSvgText(svg_text) => {
                self.svg_text = svg_text;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_clicked = ctx.link().callback(Msg::RecomputeSvg);
        html! {
            <>  
                <GraphFilter 
                    title={"Specify (optional) render settings:"} 
                    update_settings={on_clicked.clone()} 
                    dependency={ctx.props().trace_file_text.clone()}
                />
                <Graph svg_text={self.svg_text.clone()} /> 
            </>
        }
    }
}