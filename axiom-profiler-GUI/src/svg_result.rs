use self::colors::HSVColour;
use crate::{
    filter_chain::{FilterChain, Msg as FilterChainMsg},
    graph_filters::Filter, graph_container::GraphContainer,
};
use material_yew::WeakComponentLink;
use num_format::{Locale, ToFormattedString};
use petgraph::dot::{Config, Dot};
use smt_log_parser::{
    items::QuantIdx,
    parsers::{
        z3::{
            inst_graph::{EdgeType, InstGraph, InstInfo},
            z3parser::Z3Parser,
        },
        LogParser,
    },
};
use std::num::NonZeroUsize;
use viz_js::VizInstance;
use web_sys::window;
use yew::prelude::*;

pub const NODE_LIMIT: usize = 125;

pub enum Msg {
    UpdateSvgText(AttrValue),
    UpdateSelectedNode(usize),
    RenderGraph(UserPermission),
    ApplyFilter(Filter),
    ResetGraph,
    GetUserPermission,
}

pub struct UserPermission {
    permission: bool,
}

impl Default for UserPermission {
    fn default() -> Self {
        Self { permission: false }
    }
}

impl From<bool> for UserPermission {
    fn from(value: bool) -> Self {
        Self { permission: value }
    }
}

pub struct SVGResult {
    parser: Z3Parser,
    colour_map: QuantIdxToColourMap,
    inst_graph: InstGraph,
    svg_text: AttrValue,
    selected_inst: Option<InstInfo>,
    filter_chain_link: WeakComponentLink<FilterChain>,
    on_node_select: Callback<usize>,
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
            filter_chain_link: WeakComponentLink::default(),
            on_node_select: ctx.link().callback(Msg::UpdateSelectedNode),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::ApplyFilter(filter) => {
                log::debug!("Applying filter {}", filter);
                filter.apply(&mut self.inst_graph);
                false
            }
            Msg::ResetGraph => {
                log::debug!("Resetting graph");
                self.inst_graph.reset();
                false
            }
            Msg::RenderGraph(UserPermission { permission }) => {
                // as long as displayed graph contains at most 125 nodes, render time is acceptable
                log::debug!("The graph has {} nodes", self.inst_graph.node_count());
                if self.inst_graph.node_count() <= NODE_LIMIT || permission {
                    log::debug!("Rendering graph");
                    let filtered_graph = &self.inst_graph.inst_graph;
                    let dot_output = format!(
                        "{:?}",
                        Dot::with_attr_getters(
                            filtered_graph,
                            &[Config::EdgeNoLabel, Config::NodeNoLabel],
                            &|_, edge_data| format!(
                                "style={}",
                                match edge_data.weight().edge_type {
                                    EdgeType::Direct => "solid",
                                    EdgeType::Indirect => "dashed",
                                }
                            ),
                            &|_, (node_idx, node_data)| {
                                format!("label=\"{}\" style=\"{}\", shape=oval, fillcolor=\"{}\", fontcolor=black ",
                                        // node_data.line_nr,
                                        node_idx.index(),
                                        match self.inst_graph.node_has_filtered_direct_neighbours(node_idx) {
                                            true => "filled,diagonals",
                                            false => "filled",
                                        },
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
                        // options.engine = "twopi".to_string();
                        let svg = graphviz
                            .render_svg_element(dot_output, options)
                            .expect("Could not render graphviz");
                        let svg_text = svg.outer_html();
                        link.send_message(Msg::UpdateSvgText(AttrValue::from(svg_text)));
                    });
                    // only need to re-render once the new SVG has been set
                    false
                } else {
                    ctx.link().send_message(Msg::GetUserPermission);
                    false
                }
            }
            Msg::GetUserPermission => {
                log::debug!("Getting user permission");
                let window = window().unwrap();
                // Show the dialog with custom content
                let node_count = self
                    .inst_graph
                    .node_count()
                    .to_formatted_string(&Locale::en);
                let edge_count = self
                    .inst_graph
                    .edge_count()
                    .to_formatted_string(&Locale::en);
                let message = format!("Warning: The graph you are about to render contains {} nodes and {} edges, rendering might be slow. Do you want to proceed?", node_count, edge_count);
                let result = window.confirm_with_message(&message);
                match result {
                    Ok(true) => {
                        // if the user wishes to render the current graph, we do so
                        log::debug!("Got user permission");
                        ctx.link()
                            .send_message(Msg::RenderGraph(UserPermission::from(true)));
                        false
                    }
                    Ok(false) => {
                        log::debug!("Didn't get user permission");
                        // this resets the filter chain to the filter chain that we had
                        // right before adding the filter that caused too many nodes
                        // to be added to the graph
                        self.filter_chain_link
                            .borrow()
                            .clone()
                            .unwrap()
                            .send_message(FilterChainMsg::SetToPrevious);
                        false
                    }
                    Err(_) => {
                        // Handle the case where an error occurred
                        false
                    }
                }
            }
            Msg::UpdateSvgText(svg_text) => {
                log::debug!("Updating svg text");
                self.svg_text = svg_text;
                self.selected_inst = None;
                true
            }
            Msg::UpdateSelectedNode(index) => {
                log::debug!("Updating selected node");
                self.selected_inst = self.inst_graph.get_instantiation_info(index, &self.parser);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // let on_node_select = ctx.link().callback(Msg::UpdateSelectedNode);
        let node_and_edge_count_preview = html! {
            <h4>{format!{"The filtered graph contains {} nodes and {} edges", self.inst_graph.node_count(), self.inst_graph.edge_count()}}</h4>
        };
        let apply_filter = ctx.link().callback(Msg::ApplyFilter);
        let reset_graph = ctx.link().callback(|_| Msg::ResetGraph);
        let render_graph = ctx.link().callback(Msg::RenderGraph);
        html! {
            <>
                <div style="flex: 30%; height: 85vh; overflow-y: auto; ">
                <ContextProvider<Option<InstInfo>> context={self.selected_inst.clone()} >
                    <FilterChain
                        apply_filter={apply_filter.clone()}
                        reset_graph={reset_graph.clone()}
                        render_graph={render_graph.clone()}
                        weak_link={self.filter_chain_link.clone()}
                        dependency={ctx.props().trace_file_text.clone()}
                    />
                </ContextProvider<Option<InstInfo>>>
                {node_and_edge_count_preview}
                </div>
                <GraphContainer
                    svg_text={&self.svg_text}
                    update_selected_node={&self.on_node_select}
                />
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
