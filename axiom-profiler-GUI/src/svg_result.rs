use std::iter::zip;

use self::colors::{make_hsv_colours, HSVColour};
use crate::graph::Graph;
use crate::graph_filter::GraphFilter;
use fxhash::FxHashMap;
use petgraph::dot::{Config, Dot};
use smt_log_parser::{
    items::{QuantIdx, Quantifier},
    parsers::z3::results::{FilterSettings, InstGraph, InstInfo},
};
use typed_index_collections::TiVec;
use viz_js::VizInstance;
use yew::prelude::*;

pub enum Msg {
    RecomputeSvg(FilterSettings),
    UpdateSvgText(AttrValue),
    SelectedNodeIndex(usize),
}

pub struct SVGResult {
    pub svg_text: AttrValue,
    pub inst_graph: InstGraph,
    pub selected_inst: Option<InstInfo>,
    colour_map: QuantIdxToColourMap,
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(ctx: &Context<Self>) -> Self {
        let inst_graph = InstGraph::from(ctx.props().trace_file_text.as_ref());
        let quantifiers = inst_graph.get_quantifiers();
        let colour_map = QuantIdxToColourMap::from(quantifiers);
        Self {
            svg_text: AttrValue::default(),
            inst_graph,
            selected_inst: None,
            colour_map,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::RecomputeSvg(settings) => {
                let filtered_graph = self.inst_graph.filter(settings);
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
            }
            Msg::UpdateSvgText(svg_text) => {
                self.svg_text = svg_text;
                self.selected_inst = None;
                true
            }
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

type ColourIdx = usize;

struct QuantIdxToColourMap {
    colour_map: FxHashMap<QuantIdx, ColourIdx>,
    colours: TiVec<ColourIdx, HSVColour>,
}

impl QuantIdxToColourMap {
    pub fn from(quants: &TiVec<QuantIdx, Quantifier>) -> Self {
        let mut colour_map = FxHashMap::default();
        let colours = make_hsv_colours(quants.len());
        for (qidx, colour_idx) in zip(quants.keys(), colours.keys()) {
            colour_map.insert(qidx, colour_idx);
        }
        Self {
            colour_map,
            colours,
        }
    }

    pub fn get(&self, qidx: &QuantIdx) -> HSVColour {
        if let Some(colour_idx) = self.colour_map.get(qidx) {
            self.colours.raw[*colour_idx]
        } else {
            HSVColour::default()
        }
    }
}

/// Private module for generating colors
mod colors {
    use super::*;
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

    impl Default for HSVColour {
        /// The default HSV colour is white (0, 0, 1)
        fn default() -> Self {
            Self {
                hue: 0.0,
                sat: 0.0,
                val: 1.0,
            }
        }
    }

    /// Generate `n` distinct colors in HSV format
    pub fn make_hsv_colours(n: usize) -> TiVec<ColourIdx, HSVColour> {
        // want black font to be clearly visible, hence these values for saturation and value
        const DEFAULT_SAT: f64 = 0.4;
        const DEFAULT_VAL: f64 = 0.95;
        let hues: Vec<f64> = (0..n).map(|i| i as f64 / (n as f64)).collect();
        hues.iter()
            .map(|&hue| HSVColour {
                hue,
                sat: DEFAULT_SAT,
                val: DEFAULT_VAL,
            })
            .collect()
    }
}
