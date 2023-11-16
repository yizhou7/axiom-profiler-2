use std::fmt::Display;

use self::colors::HSVColour;
use crate::graph::Graph;
use crate::graph_filter::GraphFilter;
use petgraph::dot::{Config, Dot};
use smt_log_parser::{
    items::QuantIdx,
    parsers::{z3::{inst_graph::{InstGraph, InstInfo, NodeData}, z3parser::Z3Parser}, LogParser},
};
use viz_js::VizInstance;
use yew::prelude::*;

pub enum Msg {
    UpdateSvgText(AttrValue),
    SelectedNodeIndex(usize),
    AddFilter(Filter),
    RemoveNthFilter(usize),
    RemoveAllFilters,
    RenderGraph,
}

pub struct SVGResult {
    parser: Z3Parser,
    colour_map: QuantIdxToColourMap,
    orig_graph: InstGraph,
    filter_chain: Vec<Filter>,
    inst_graph: InstGraph,
    svg_text: AttrValue,
    selected_inst: Option<InstInfo>,
    node_count_preview: usize,
}

#[derive(Clone, Copy)]
pub enum Filter {
    MaxLineNr(usize),
    IgnoreTheorySolving,
    MaxInsts(usize),
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxLineNr(line_nr) => write!(f, "Only show up to line number {}", line_nr),
            Self::IgnoreTheorySolving => write!(f, "Ignore theory solving instantiations"),
            Self::MaxInsts(max) => write!(f, "Show the {} most expensive instantiations", max)
        }
    }
}

impl Filter {
    fn apply(self: Filter, graph: &mut InstGraph) {
        match self {
            Filter::MaxLineNr(max_line_nr) => {
                graph.retain_nodes(|node: &NodeData| node.line_nr <= max_line_nr)
            }, 
            Filter::IgnoreTheorySolving => {
                // want to use retain_nodes_and_reconnect eventually 
                graph.retain_nodes(|node: &NodeData| !node.is_theory_inst)
            },
            Filter::MaxInsts(n) => {
                graph.keep_n_most_costly(n);
            }
        }
    }
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
        let mut inst_graph = InstGraph::from(&parser);
        let total_nr_of_quants = parser.total_nr_of_quants();
        let colour_map = QuantIdxToColourMap::from(total_nr_of_quants);
        let filter_chain = vec![Filter::IgnoreTheorySolving, Filter::MaxInsts(250)];
        // apply initial filters
        for &filter in &filter_chain {
            filter.apply(&mut inst_graph);
        }
        let node_count_preview = inst_graph.node_count();
        Self {
            parser,
            colour_map,
            orig_graph,
            filter_chain,
            inst_graph,
            svg_text: AttrValue::default(),
            selected_inst: None,
            node_count_preview,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddFilter(filter) => {
                self.filter_chain.push(filter);
                // TODO: apply filter that was just added to currently displayed graph 
                // and compute how many nodes the current graph has such that user can
                // decide to render it or not
                filter.apply(&mut self.inst_graph);
                self.node_count_preview = self.inst_graph.node_count();
                true
            },
            Msg::RemoveNthFilter(n) => {
                self.filter_chain.remove(n);
                self.inst_graph = self.orig_graph.clone();
                for filter in &self.filter_chain {
                    filter.apply(&mut self.inst_graph);
                }
                self.node_count_preview = self.inst_graph.node_count();
                true
            },
            Msg::RemoveAllFilters => {
                self.filter_chain = Vec::new();
                self.inst_graph = self.orig_graph.clone();
                self.node_count_preview = self.inst_graph.node_count();
                true
            }
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
        let on_clicked = ctx.link().callback(Msg::AddFilter);
        let on_node_select = ctx.link().callback(Msg::SelectedNodeIndex);
        let remove_all_filters = ctx.link().callback(|_| Msg::RemoveAllFilters);
        let filter_chain: Vec<yew::virtual_dom::VNode> = self.filter_chain
            .iter()
            .enumerate()
            .map(|(idx, f)| html! {
                <div>
                    <p>{format!("{}. {f}", idx+1)}</p>
                    <button onclick={ctx.link().callback(move |_| Msg::RemoveNthFilter(idx))}>{"Remove filter"}</button>
                </div>
            })
            .collect();
        let render_graph = ctx.link().callback(|_| Msg::RenderGraph);
        let node_count_preview = html! {
            <h4>{format!{"The filtered graph contains {} nodes", self.node_count_preview}}</h4>
        };
        html! {
            <>
                <GraphFilter
                    title={"Specify (optional) render settings:"}
                    update_settings={on_clicked.clone()}
                    dependency={ctx.props().trace_file_text.clone()}
                />
                <h3>{"Filter chain:"}</h3>
                {for filter_chain}
                {node_count_preview}
                <button onclick={remove_all_filters}>{"Remove all filters"}</button>
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
