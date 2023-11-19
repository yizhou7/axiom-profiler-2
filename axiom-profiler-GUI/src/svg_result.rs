use std::num::NonZeroUsize;
use self::colors::HSVColour;
use crate::{graph::Graph, filter_chain::FilterChain, graph_filters::Filter};
use petgraph::{dot::{Config, Dot}, Direction::{Outgoing, Incoming, self}};
use smt_log_parser::{
    items::QuantIdx,
    parsers::{z3::{inst_graph::{InstGraph, InstInfo}, z3parser::Z3Parser}, LogParser},
};
use viz_js::VizInstance;
use yew::prelude::*;
use petgraph::graph::NodeIndex;

pub enum Msg {
    UpdateSvgText(AttrValue),
    SelectedNodeIndex(usize),
    RenderGraph,
    ExplicitRender,
    ApplyFilter(Filter),
    ResetGraph,
    HideSelectedNode,
    ShowNeighbours(Direction),
    ShowSourceTree,
}

pub struct SVGResult {
    parser: Z3Parser,
    colour_map: QuantIdxToColourMap,
    inst_graph: InstGraph,
    svg_text: AttrValue,
    selected_inst: Option<InstInfo>,
    explicit_render: bool,
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
        let inst_graph = InstGraph::from(&parser);
        let total_nr_of_quants = parser.total_nr_of_quants();
        let colour_map = QuantIdxToColourMap::from(total_nr_of_quants);
        Self {
            parser,
            colour_map,
            inst_graph,
            svg_text: AttrValue::default(),
            selected_inst: None,
            explicit_render: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ApplyFilter(filter) => {
                filter.apply(&mut self.inst_graph);
                ctx.link().send_message(Msg::RenderGraph);
                false
            },
            Msg::ResetGraph => {
                self.inst_graph.reset();
                ctx.link().send_message(Msg::RenderGraph);
                false 
            },
            Msg::RenderGraph => {
                let filtered_graph = &self.inst_graph.inst_graph;
                // as long as displayed graph contains at most 250, render time is acceptable
                if self.explicit_render || self.inst_graph.node_count() <= 250 {
                    // need to reset the explicit user permission to render after each explicit render
                    self.explicit_render = false;
                    let dot_output = format!(
                        "{:?}",
                        Dot::with_attr_getters(
                            filtered_graph,
                            &[Config::EdgeNoLabel, Config::NodeNoLabel],
                            &|_, _| String::new(),
                            &|_, (_, node_data)| {
                                format!("label=\"{}\" style=filled, shape=oval, fillcolor=\"{}\", fontcolor=black ",
                                    node_data.line_nr,
                                    self.colour_map.get(&node_data.quant_idx)
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
                } else {
                    // in case there is no explicit render-permission and the graph contains more than 250 nodes
                    // need to display warning and get explicit user-permission
                    true
                }
            },
            Msg::ExplicitRender => {
                self.explicit_render = true;
                ctx.link().send_message(Msg::RenderGraph);
                false
            }
            Msg::UpdateSvgText(svg_text) => {
                self.svg_text = svg_text;
                self.selected_inst = None;
                true
            }
            Msg::SelectedNodeIndex(index) => {
                self.selected_inst = self.inst_graph.get_instantiation_info(index, &self.parser);
                true
            },
            Msg::HideSelectedNode => {
                self.inst_graph.remove_subtree_with_root(self.selected_inst.as_ref().unwrap().node_index.clone());
                ctx.link().send_message(Msg::RenderGraph);
                false
            },
            Msg::ShowNeighbours(direction) => {
                self.inst_graph.show_neighbours(self.selected_inst.as_ref().unwrap().node_index.clone(), direction);
                ctx.link().send_message(Msg::RenderGraph);
                false
            },
            Msg::ShowSourceTree => {
                self.inst_graph.only_show_ancestors(self.selected_inst.as_ref().unwrap().node_index.clone());
                ctx.link().send_message(Msg::RenderGraph);
                false
            }
        }
    }


    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_node_select = ctx.link().callback(Msg::SelectedNodeIndex);
        let explicit_render = ctx.link().callback(|_| Msg::ExplicitRender);
        let node_count_preview = html! {
            <h4>{format!{"The filtered graph contains {} nodes", self.inst_graph.node_count()}}</h4>
        };
        let apply_filter = ctx.link().callback(Msg::ApplyFilter);
        let reset_graph = ctx.link().callback(|_| Msg::ResetGraph);
        let hide_node = ctx.link().callback(|_| Msg::HideSelectedNode);
        let show_neighbours = ctx.link().callback(Msg::ShowNeighbours);
        let show_children = {
            let show = show_neighbours.clone();
            Callback::from(move |_| show.emit(Outgoing))
        };
        let show_parents = {
            let show = show_neighbours.clone();
            Callback::from(move |_| show.emit(Incoming))
        };
        let show_source_tree = ctx.link().callback(|_| Msg::ShowSourceTree);
        html! {
            <>
                <FilterChain apply_filter={apply_filter.clone()} reset_graph={reset_graph.clone()} dependency={ctx.props().trace_file_text.clone()}/>
                {node_count_preview}
                {if self.inst_graph.node_count() > 250 {
                    html! {
                        <>
                            <h4>
                                {
                                    "Warning: The current graph contains a large number of nodes, rendering might be slow.\n
                                    Render anyways?"
                                }
                            </h4>
                            <button onclick={explicit_render}>{"Yes"}</button>
                        </>
                    }
                } else {
                    html!{}
                }}
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
                            <button onclick={hide_node}>{"Hide selected node and its descendants"}</button>
                            <button onclick={show_children}>{"Show children of selected node"}</button>
                            <button onclick={show_parents}>{"Show parents of selected node"}</button>
                            <button onclick={show_source_tree}>{"Only show ancestors of selected node"}</button>
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
    coprime: NonZeroUsize,
    shift: usize,
}

impl QuantIdxToColourMap {
    pub fn from(total_nr_of_quants: usize) -> Self {
        Self {
            total_nr_of_quants,
            coprime: Self::find_coprime(total_nr_of_quants),
            // Currently `idx == 0` will always have the same hue of 0, if we do
            // not want this behavior pick a random number here instead.
            shift: 0,
        }
    }

    pub fn get(&self, qidx: &QuantIdx) -> HSVColour {
        let idx = usize::from(*qidx);
        debug_assert!(idx < self.total_nr_of_quants);
        let idx_perm = (idx * self.coprime.get() + self.shift) % self.total_nr_of_quants;
        HSVColour {
            hue: idx_perm as f64 / self.total_nr_of_quants as f64,
            sat: 0.4,
            val: 0.95,
        }
    }

    fn find_coprime(n: usize) -> NonZeroUsize {
        // Workaround since `unwrap` isn't allowed in const functions.
        const ONE: NonZeroUsize = match NonZeroUsize::new(1) {
            Some(nz) => nz,
            None => [][0],
        };
        let nz = NonZeroUsize::new(n);
        if let Some(nz) = nz {
            primal::Primes::all()
                // Start from 13 since the smaller ones don't permute so nicely.
                .skip(5)
                // SAFETY: returned primes will never be zero.
                .map(|p| unsafe { NonZeroUsize::new_unchecked(p) })
                // Find the first prime that is coprime to `nz`.
                .find(|&prime| nz.get() % prime.get() != 0)
                // Will always succeed since any prime larger than `nz / 2` is
                // coprime. Terminates since `nz != 0`.
                .unwrap()
        } else {
            ONE
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
