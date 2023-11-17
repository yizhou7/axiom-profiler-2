use self::colors::HSVColour;
use crate::{graph::Graph, filter_chain::FilterChain, graph_filters::Filter};
use petgraph::dot::{Config, Dot};
use smt_log_parser::{
    items::QuantIdx,
    parsers::{z3::{inst_graph::{InstGraph, InstInfo}, z3parser::Z3Parser}, LogParser},
};
use viz_js::VizInstance;
use yew::prelude::*;

pub enum Msg {
    UpdateSvgText(AttrValue),
    SelectedNodeIndex(usize),
    RenderGraph,
    ApplyFilter(Filter),
    ResetGraph,
}

pub struct SVGResult {
    parser: Z3Parser,
    colour_map: QuantIdxToColourMap,
    orig_graph: InstGraph,
    inst_graph: InstGraph,
    svg_text: AttrValue,
    selected_inst: Option<InstInfo>,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(ctx: &Context<Self>) -> Self {
        let parser = Z3Parser::from_str(&ctx.props().trace_file_text).process_all();
        let orig_graph = InstGraph::from(&parser);
        let inst_graph = InstGraph::from(&parser);
        let total_nr_of_quants = parser.total_nr_of_quants();
        let colour_map = QuantIdxToColourMap::from(total_nr_of_quants);
        Self {
            parser,
            colour_map,
            orig_graph,
            inst_graph,
            svg_text: AttrValue::default(),
            selected_inst: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ApplyFilter(filter) => {
                filter.apply(&mut self.inst_graph);
                true
            },
            Msg::ResetGraph => {
                self.inst_graph = self.orig_graph.clone();
                true
            },
            Msg::RenderGraph => {
                let filtered_graph = &self.inst_graph.inst_graph;
                let dot_output = format!(
                    "{:?}",
                    Dot::with_attr_getters(
                        filtered_graph,
                        &[Config::EdgeNoLabel, Config::NodeNoLabel],
                        &|_, _| String::new(),
                        &|_, (_, node_data)| {
                            format!("label=\"{}\" style=filled, shape=oval, fillcolor=\"{}\", fontcolor=black ",
                                node_data.line_nr,
                                self.colour_map.get(&node_data.quant_idx).unwrap() 
                            )
                        },
                    )
                );
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
                });
                // only need to re-render once the new SVG has been set
                false
            },
            Msg::UpdateSvgText(svg_text) => {
                self.svg_text = svg_text;
                self.selected_inst = None;
                true
            }
            Msg::SelectedNodeIndex(index) => {
                self.selected_inst = self.inst_graph.get_instantiation_info(index, &self.parser);
                true
            },
        }
    }


    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_node_select = ctx.link().callback(Msg::SelectedNodeIndex);
        let render_graph = ctx.link().callback(|_| Msg::RenderGraph);
        let node_count_preview = html! {
            <h4>{format!{"The filtered graph contains {} nodes", self.inst_graph.node_count()}}</h4>
        };
        let apply_filter = ctx.link().callback(Msg::ApplyFilter);
        let reset_graph = ctx.link().callback(|_| Msg::ResetGraph);
        html! {
            <>
                <FilterChain apply_filter={apply_filter.clone()} reset_graph={reset_graph.clone()} dependency={ctx.props().trace_file_text.clone()}/>
                {node_count_preview}
                <button onclick={render_graph}>{"Render graph"}</button>
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

struct QuantIdxToColourMap {
    total_nr_of_quants: usize,
}

impl QuantIdxToColourMap {
    pub fn from(total_nr_of_quants: usize) -> Self {
        Self {
            total_nr_of_quants,
        }
    }

    pub fn get(&self, qidx: &QuantIdx) -> Option<HSVColour> {
        let idx = usize::from(*qidx);
        if idx < self.total_nr_of_quants {
            Some(HSVColour {
                hue: idx as f64 / (self.total_nr_of_quants as f64),
                sat: 0.4,
                val: 0.95,
            })
        } else {
            None
        }
    }
}

/// Private module for generating colors
mod colors {
    use std::fmt;

    #[derive(Clone, Copy)]
    pub struct HSVColour {
        pub hue: f64,
        pub sat: f64,
        pub val: f64,
    }

    impl fmt::Display for HSVColour {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{} {} {}", self.hue, self.sat, self.val)
        }
    }
}
