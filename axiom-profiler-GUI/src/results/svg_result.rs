use crate::{
    results::{filters::graph_filters::FilterOutput, graph_info::{GraphInfo, Msg as GraphInfoMsg}}, OpenedFileInfo, RcParser
};

use self::colours::HSVColour;
use super::{
    filters::{
        filter_chain::{FilterChain, Msg as FilterChainMsg},
        graph_filters::Filter,
    },
    worker::Worker,
};
use gloo::console::log;
use material_yew::WeakComponentLink;
use num_format::{Locale, ToFormattedString};
use petgraph::dot::{Config, Dot};
use petgraph::graph::EdgeIndex;
use smt_log_parser::{
    items::{BlameKind, InstIdx, MatchKind, QuantIdx},
    parsers::z3::inst_graph::{EdgeInfo, EdgeType, InstGraph, InstInfo, VisibleGraphInfo},
};
use std::num::NonZeroUsize;
use viz_js::VizInstance;
use web_sys::window;
use yew::prelude::*;

pub const EDGE_LIMIT: usize = 2000;
pub const DEFAULT_NODE_COUNT: usize = 200;
pub const NODE_COLOUR_SATURATION: f64 = 0.4;
pub const NODE_COLOUR_VALUE: f64 = 0.95;

pub enum Msg {
    ConstructedGraph(SVGData),
    UpdateSvgText(AttrValue, bool),
    RenderGraph(UserPermission),
    ApplyFilter(Filter),
    ResetGraph,
    GetUserPermission,
    WorkerOutput(super::worker::WorkerOutput),
    UpdateSelectedNodes(Vec<InstInfo>),
    SearchMatchingLoops,
    SelectNthMatchingLoop(usize),
    ShowMatchingLoopSubgraph,
}

#[derive(Default)]
pub struct UserPermission {
    permission: bool,
}

impl From<bool> for UserPermission {
    fn from(value: bool) -> Self {
        Self { permission: value }
    }
}

struct GraphDimensions {
    node_count: usize,
    edge_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderingState {
    ConstructingGraph,
    ConstructedGraph,
    GraphToDot,
    RenderingGraph,
}

pub struct SVGResult {
    colour_map: QuantIdxToColourMap,
    svg_text: AttrValue,
    filter_chain_link: WeakComponentLink<FilterChain>,
    insts_info_link: WeakComponentLink<GraphInfo>,
    graph_dim: GraphDimensions,
    worker: Option<Box<dyn yew_agent::Bridge<Worker>>>,
    async_graph_and_filter_chain: bool,
    selected_insts: Vec<InstInfo>,
    data: Option<SVGData>,
    queue: Vec<Msg>,
}

pub struct SVGData {
    get_node_info: Callback<(InstIdx, bool, RcParser), InstInfo>,
    get_edge_info: Callback<(EdgeIndex, bool, RcParser), EdgeInfo>,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub file: OpenedFileInfo,
    pub progress: Callback<Option<RenderingState>>,
    pub selected_insts_cb: Callback<Vec<(InstIdx, Option<QuantIdx>)>>,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(ctx: &Context<Self>) -> Self {
        log::debug!("Creating SVGResult component");
        let old = std::mem::replace(&mut *ctx.props().file.update.borrow_mut(), Ok(ctx.link().callback(|msg| msg)));
        if let Err(old) = old {
            ctx.link().send_message_batch(old);
        }
        let parser = &ctx.props().file.parser;
        ctx.props().progress.emit(Some(RenderingState::ConstructingGraph));
        let (quant_count, non_quant_insts) = parser.borrow().quant_count_incl_theory_solving();
        let colour_map = QuantIdxToColourMap::from(quant_count, non_quant_insts);
        let link = ctx.link().clone();
        let parser = RcParser::clone(parser);
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(10).await;
            let inst_graph = InstGraph::from(&parser.borrow());
            let get_node_info = Callback::from({
                let node_info_map = inst_graph.get_node_info_map();
                move |(node, ignore_ids, parser): (InstIdx, bool, RcParser)| {
                    node_info_map.get_instantiation_info(node, &parser.borrow(), ignore_ids)
                }
            });
            let get_edge_info = Callback::from({
                let edge_info_map = inst_graph.get_edge_info_map();
                move |(edge, ignore_ids, parser): (EdgeIndex, bool, RcParser)| {
                    edge_info_map.get_edge_info(edge, &parser.borrow(), ignore_ids)
                }
            });
            let _ = parser.graph.borrow_mut().insert(inst_graph);
            link.send_message(Msg::ConstructedGraph(SVGData { get_node_info, get_edge_info }));
        });
        Self {
            colour_map,
            svg_text: AttrValue::default(),
            filter_chain_link: WeakComponentLink::default(),
            insts_info_link: WeakComponentLink::default(),
            graph_dim: GraphDimensions {
                node_count: 0,
                edge_count: 0,
            },
            worker: Some(Self::create_worker(ctx.link().clone())),
            async_graph_and_filter_chain: false,
            selected_insts: Vec::new(),
            data: None,
            queue: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        if let Msg::ConstructedGraph(data) = msg {
            let _ = self.data.insert(data);
            let queue = std::mem::replace(&mut self.queue, Vec::new());
            ctx.props().progress.emit(Some(RenderingState::ConstructedGraph));
            ctx.link().send_message_batch(queue);
            return true;
        }
        let mut inst_graph = ctx.props().file.parser.graph.borrow_mut();
        let Some(inst_graph) = inst_graph.as_mut() else {
            self.queue.push(msg);
            return false;
        };
        match msg {
            Msg::ConstructedGraph(_) => unreachable!(),
            Msg::WorkerOutput(_out) => false,
            Msg::ApplyFilter(filter) => {
                log::debug!("Applying filter {}", filter);
                match filter.apply(inst_graph, &mut ctx.props().file.parser.borrow_mut()) {
                    FilterOutput::LongestPath(path) => {
                        self.insts_info_link
                            .borrow()
                            .clone()
                            .unwrap()
                            .send_message(GraphInfoMsg::SelectNodes(path));
                        false
                    }
                    FilterOutput::MatchingLoopGeneralizedTerms(gen_terms) => {
                        self.insts_info_link
                            .borrow()
                            .clone()
                            .unwrap()
                            .send_message(GraphInfoMsg::ShowGeneralizedTerms(gen_terms));
                        false
                    }
                    FilterOutput::None => false
                }
            }
            Msg::SearchMatchingLoops => {
                inst_graph.search_matching_loops();
                ctx.link().send_message(Msg::SelectNthMatchingLoop(0));
                true
            }
            Msg::SelectNthMatchingLoop(n) => {
                self.filter_chain_link
                    .borrow()
                    .clone()
                    .unwrap()
                    .send_message(FilterChainMsg::AddFilters(vec![Filter::SelectNthMatchingLoop(n)]));
                false
            }
            Msg::ShowMatchingLoopSubgraph => {
                self.filter_chain_link
                    .borrow()
                    .clone()
                    .unwrap()
                    .send_message(FilterChainMsg::AddFilters(vec![Filter::ShowMatchingLoopSubgraph]));
                false
            }
            Msg::ResetGraph => {
                log::debug!("Resetting graph");
                inst_graph.reset_visibility_to(true);
                false
            }
            Msg::RenderGraph(UserPermission { permission }) => {
                let VisibleGraphInfo {
                    node_count,
                    edge_count,
                    node_count_decreased,
                    edge_count_decreased,
                } = inst_graph.retain_visible_nodes_and_reconnect();
                log::debug!("The current node count is {}", node_count);
                self.graph_dim.node_count = node_count;
                self.graph_dim.edge_count = edge_count;
                let safe_to_render = edge_count <= EDGE_LIMIT
                    || node_count <= DEFAULT_NODE_COUNT
                    || edge_count_decreased
                    || node_count_decreased;
                if safe_to_render || permission {
                    self.async_graph_and_filter_chain = false;
                    log::debug!("Rendering graph");
                    ctx.props().progress.emit(Some(RenderingState::GraphToDot));
                    let filtered_graph = &inst_graph.visible_graph;

                    // Performance observations (default value is in [])
                    //  - splines=false -> 38s | [splines=true] -> ??
                    //  - nslimit=2 -> 7s | nslimit=4 -> 9s | nslimit=7 -> 11.5s | nslimit=10 -> 14s | [nslimit=INT_MAX] -> 38s
                    //  - [mclimit=1] -> 7s | mclimit=0.5 -> 4s (with nslimit=2)
                    // `ranksep` dictates the distance between ranks (rows) in the graph,
                    // it should be set dynamically based on the average number of children
                    // per node out of all nodes with at least one child.
                    let settings = [
                        "ranksep=1.0;",
                        "splines=false;",
                        "nslimit=6;",
                        "mclimit=0.6;",
                    ];
                    let dot_output = format!(
                        "digraph {{\n{}\n{:?}\n}}",
                        settings.join("\n"),
                        Dot::with_attr_getters(
                            filtered_graph,
                            &[
                                Config::EdgeNoLabel,
                                Config::NodeNoLabel,
                                Config::GraphContentOnly
                            ],
                            &|_, edge_data| format!(
                                "id={} style={} class={} arrowhead={}",
                                match edge_data.weight() {
                                    EdgeType::Direct { orig_graph_idx, .. } =>
                                        format!("edge{}", orig_graph_idx.index()),
                                    EdgeType::Indirect => "indirect".to_string(),
                                },
                                match edge_data.weight() {
                                    EdgeType::Direct { .. } => "solid",
                                    EdgeType::Indirect => "dashed",
                                },
                                match edge_data.weight() {
                                    EdgeType::Direct { .. } => "direct",
                                    EdgeType::Indirect => "indirect",
                                },
                                match edge_data.weight() {
                                    EdgeType::Direct {
                                        kind: BlameKind::Equality { .. },
                                        ..
                                    } => "empty",
                                    _ => "normal",
                                }
                            ),
                            &|_, (_, node_data)| {
                                format!("id=node{} label=\"{}\" style=\"{}\" shape={} fillcolor=\"{}\" fontcolor=black gradientangle=90",
                                        usize::from(node_data.inst_idx),
                                        usize::from(node_data.inst_idx),
                                        if node_data.mkind.is_mbqi() { "filled,dashed" } else { "filled" },
                                        // match (self.inst_graph.node_has_filtered_children(node_data.orig_graph_idx), 
                                        //        self.inst_graph.node_has_filtered_parents(node_data.orig_graph_idx)) {
                                        //     (false, false) => format!("{}", self.colour_map.get(&node_data.quant_idx, 0.7)),
                                        //     (false, true) => format!("{}:{}", self.colour_map.get(&node_data.quant_idx, 1.0), self.colour_map.get(&node_data.quant_idx, 0.1)),
                                        //     (true, false) => format!("{}:{}", self.colour_map.get(&node_data.quant_idx, 0.1), self.colour_map.get(&node_data.quant_idx, 1.0)),
                                        //     (true, true) => format!("{}", self.colour_map.get(&node_data.quant_idx, 0.3)),
                                        // },
                                        match (inst_graph.node_has_filtered_children(node_data.inst_idx),
                                               inst_graph.node_has_filtered_parents(node_data.inst_idx)) {
                                            (false, false) => "box",
                                            (false, true) => "house",
                                            (true, false) => "invhouse",
                                            (true, true) => "diamond",
                                        },
                                        self.colour_map.get(&node_data.mkind, NODE_COLOUR_SATURATION),
                                    )
                            },
                        )
                    );
                    log::debug!("Finished building dot output");
                    ctx.props().progress.emit(Some(RenderingState::RenderingGraph));
                    let link = ctx.link().clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(10).await;
                        let graphviz = VizInstance::new().await;
                        let options = viz_js::Options::default();
                        // options.engine = "twopi".to_string();
                        let svg = graphviz
                            .render_svg_element(dot_output, options)
                            .expect("Could not render graphviz");
                        let svg_text = svg.outer_html();
                        link.send_message(Msg::UpdateSvgText(
                            AttrValue::from(svg_text),
                            node_count_decreased,
                        ));
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
                let node_count = self.graph_dim.node_count.to_formatted_string(&Locale::en);
                let edge_count = self.graph_dim.edge_count.to_formatted_string(&Locale::en);
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
                        let message = "Would you like to apply the filter without rendering?";
                        let result = window.confirm_with_message(message);
                        match result {
                            Ok(true) => {
                                self.async_graph_and_filter_chain = true;
                                true
                            }
                            Ok(false) => {
                                self.filter_chain_link
                                    .borrow()
                                    .clone()
                                    .unwrap()
                                    .send_message(FilterChainMsg::SetToPrevious);
                                false
                            }
                            Err(_) => false,
                        }
                    }
                    Err(_) => {
                        // Handle the case where an error occurred
                        false
                    }
                }
            }
            Msg::UpdateSvgText(svg_text, node_count_decreased) => {
                log::debug!("Updating svg text");
                ctx.props().progress.emit(None);
                if svg_text != self.svg_text {
                    self.svg_text = svg_text;
                    // only if some nodes were deleted, do we deselect all previously selected nodes
                    if node_count_decreased {
                        self.insts_info_link
                            .borrow()
                            .clone()
                            .unwrap()
                            .send_message(GraphInfoMsg::DeselectAll);
                    }
                    true
                } else {
                    true
                }
            }
            Msg::UpdateSelectedNodes(nodes) => {
                self.selected_insts = nodes;
                let insts = self.selected_insts.iter().map(|i| (i.inst_idx, i.mkind.quant_idx())).collect();
                ctx.props().selected_insts_cb.emit(insts);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let Some(data) = &self.data else {
            return html! {};
        };
        let update_selected_nodes = ctx.link().callback(Msg::UpdateSelectedNodes);
        html! {
            <GraphInfo
                weak_link={self.insts_info_link.clone()}
                node_info={data.get_node_info.clone()}
                edge_info={data.get_edge_info.clone()}
                parser={ctx.props().file.parser.clone()}
                svg_text={&self.svg_text}
                outdated={self.async_graph_and_filter_chain}
                {update_selected_nodes}
            />
        }
    }
}

impl SVGResult {
    /// Deletes the old worker with its queue of messages and creates a new one.
    /// Any enqueued work will still continue to run (there is no way to cancel this
    /// at the moment, see https://github.com/rustwasm/gloo/issues/408) but will not
    /// send a `WorkerOutput` message on completion.
    pub fn reset_worker(&mut self, link: yew::html::Scope<Self>) {
        // The old worker is dropped when overwritten here. Not sure we need the option?
        self.worker = Some(Self::create_worker(link));
    }
    /// Sends an input to the worker to process.
    pub fn send_worker_input(&mut self, input: super::worker::WorkerInput) {
        self.worker.as_mut().unwrap().send(input);
    }

    /// Used internally.
    fn create_worker(link: yew::html::Scope<Self>) -> Box<dyn yew_agent::Bridge<Worker>> {
        use yew_agent::Bridged;
        let cb = std::rc::Rc::new(move |e| link.send_message(Msg::WorkerOutput(e)));
        Worker::bridge(cb)
    }
}

struct QuantIdxToColourMap {
    total_count: usize,
    non_quant_insts: bool,
    coprime: NonZeroUsize,
    shift: usize,
}

impl QuantIdxToColourMap {
    pub fn from(quant_count: usize, non_quant_insts: bool) -> Self {
        let total_count = quant_count + non_quant_insts as usize;
        Self {
            total_count,
            non_quant_insts,
            coprime: Self::find_coprime(total_count),
            // Currently `idx == 0` will always have the same hue of 0, if we do
            // not want this behavior pick a random number here instead.
            shift: 0,
        }
    }

    pub fn get(&self, mkind: &MatchKind, sat: f64) -> HSVColour {
        let qidx = mkind.quant_idx();
        debug_assert!(self.non_quant_insts || qidx.is_some());
        let idx = qidx
            .map(usize::from)
            .map(|q| q + self.non_quant_insts as usize)
            .unwrap_or(0);
        // debug_assert!(idx < idx);
        let idx_perm = (idx * self.coprime.get() + self.shift) % self.total_count;
        HSVColour {
            hue: idx_perm as f64 / self.total_count as f64,
            sat,
            val: NODE_COLOUR_VALUE,
        }
    }

    fn find_coprime(n: usize) -> NonZeroUsize {
        // Workaround since `unwrap` isn't allowed in const functions.
        const ONE: NonZeroUsize = match NonZeroUsize::new(1) {
            Some(nz) => nz,
            None => [][0],
        };
        let nz = NonZeroUsize::new(n);
        // TODO: there are two bugs here:
        //   - We try to calculate the number of primes less than or equal to `n/2` as `(n/ln(n)) / 2`, rather than
        //     `n/2 / ln(n/2)` which actually gets closer to the real number (but still underapproximates).
        //   - Getting a prime as close to `n/2` as possible is not ideal since
        //     e.g. idx 0 and 2 will be right next to each other (0 -> 0, 1 -> n/2, 2 -> 1, ...).
        //     Luckily we generally get a number somewhat smaller than `n/2` so it's not too bad.
        if let Some(nz) = nz {
            // we try to find a prime close to n/10 such that 0 -> 0, 1 -> n/10, 2 -> 2n/10, ..., 10 -> 1, ...
            // this way, the indices within a window of size 10 have quite distinct colors. 
            // For small n, n/10 is close to 0 and hence we would choose a small prime which would again lead to 
            // visually indistinguishable colors. Therefore, we take the max of the prime close to n/10 and the 6-th prime
            // It is a compromise to have easily distinguishable colors for both large and small n.
            let nr_primes_to_skip = primal::StreamingSieve::prime_pi(((n as f64)/10.0).floor() as usize).max(6); 
            primal::Primes::all()
                // Start from "middle prime" smaller than n since both the very large and very small ones don't permute so nicely.
                // .skip((nr_primes_smaller_than_n / 2.0).ceil() as usize)
                .skip(nr_primes_to_skip)
                // SAFETY: returned primes will never be zero.
                .map(|p| unsafe { NonZeroUsize::new_unchecked(p) })
                // Find the first prime that is coprime to `nz`.
                .find(|&prime| nz.get() % prime.get() != 0)
                .unwrap()
            // Will always succeed since any prime larger than `nz / 2` is
            // coprime. Terminates since `nz != 0`.
        } else {
            ONE
        }
    }
}

/// Private module for generating colors
mod colours {
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
