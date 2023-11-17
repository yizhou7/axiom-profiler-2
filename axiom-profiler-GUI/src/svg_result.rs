use std::num::NonZeroUsize;
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
    ExplicitRender,
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
                self.inst_graph = self.orig_graph.clone();
                ctx.link().send_message(Msg::RenderGraph);
                false 
            },
            Msg::RenderGraph => {
                let filtered_graph = &self.inst_graph.inst_graph;
                if self.explicit_render || self.inst_graph.node_count() <= 250 {
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
        html! {
            <>
                <FilterChain apply_filter={apply_filter.clone()} reset_graph={reset_graph.clone()} dependency={ctx.props().trace_file_text.clone()}/>
                {node_count_preview}
                {if self.inst_graph.node_count() > 250 {
                    html! {
                        <>
                            <h4>
                                {
                                    "Warning: The current graph contains lots of nodes, this might take a while to render.\n
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
