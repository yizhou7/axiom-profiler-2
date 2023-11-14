use yew::prelude::*;
use smt_log_parser::parsers::z3::results::{FilterSettings, InstGraph, InstInfo};
use crate::graph_filter::GraphFilter;
use crate::graph::Graph;
use petgraph::dot::{Dot, Config};
use viz_js::VizInstance;

pub enum Msg {
    RecomputeSvg(FilterSettings),
    UpdateSvgText(AttrValue),
    SelectedNodeIndex(usize),
}

pub struct SVGResult {
    pub svg_text: AttrValue,
    pub inst_graph: InstGraph,
    pub selected_inst: Option<InstInfo>,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            svg_text: AttrValue::default(),
            inst_graph: InstGraph::from(ctx.props().trace_file_text.as_ref()), 
            selected_inst: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RecomputeSvg(settings) => {
                let filtered_graph = self.inst_graph.filter(settings);
                let dot_output = format!("{:?}", Dot::with_attr_getters(
                    filtered_graph,
                    &[Config::EdgeNoLabel, Config::NodeNoLabel],
                    &|_, _| String::new(),
                    &|_, (_, node_data)| format!("{node_data}"),
                )); 
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
                self.selected_inst = None;
                true
            },
            Msg::SelectedNodeIndex(index) => {
                self.selected_inst = self.inst_graph.get_instantiation_info(index);
                true 
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_clicked = ctx.link().callback(Msg::RecomputeSvg);
        let on_node_select = ctx.link().callback(Msg::SelectedNodeIndex);
        html! {
            <>  
                <GraphFilter 
                    title={"Specify (optional) render settings:"} 
                    update_settings={on_clicked.clone()} 
                    dependency={ctx.props().trace_file_text.clone()}
                />
                <Graph 
                    svg_text={self.svg_text.clone()}
                    update_selected_node={on_node_select.clone()} 
                /> 
                <div style="width: 50%; float: left;">
                    {if let Some(inst_info) = &self.selected_inst {
                        html! {
                            <>
                            <h2>{"Information about selected node:"}</h2>
                            <ul>
                                <li><h4>{"Instantiation happens at line number: "}</h4><p>{inst_info.inst.line_no}</p></li>
                                <li><h4>{"Cost: "}</h4><p>{inst_info.inst.cost}</p></li>
                                <li><h4>{"Instantiated formula: "}</h4><p>{inst_info.formula.clone()}</p></li>
                                // <li><h4>{"Bound terms: "}</h4>{for &inst_info.bound_terms}</li>
                                // <li><h4>{"Yield terms: "}</h4>{for &inst_info.yields_terms}</li>
                                // <li><h4>{"Variable binding information: "}</h4></li>
                                // <li><h4>{"Involved equalities: "}</h4></li>
                            </ul>
                            </>
                        }
                    } else {
                        html!{}
                    }}
                </div>
            </>
        }
    }
}