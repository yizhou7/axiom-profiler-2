use material_yew::icon::MatIcon;
use smt_log_parser::items::{InstIdx, QuantIdx};
use yew::{function_component, html, Callback, Html, MouseEvent, Properties};

use crate::{results::{filters::graph_filters::Filter, svg_result::DEFAULT_NODE_COUNT}, RcParser};

#[derive(PartialEq, Properties)]
pub struct AddFilterSidebarProps {
    pub new_filter: Callback<Filter>,
    pub found_mls: Option<usize>,
    pub insts: Vec<(InstIdx, Option<QuantIdx>)>,
    pub parser: Option<RcParser>,
}

#[function_component]
pub fn AddFilterSidebar(props: &AddFilterSidebarProps) -> Html {
    let filters = if let Some(parser) = &props.parser {
        let graph = parser.graph.borrow();
        let Some(graph) = graph.as_ref() else {
            return html!{}
        };
        vec![
            props.insts.clone().into_iter()
                .filter(|&(i, _)| graph.orig_graph.neighbors_directed(i.into(), petgraph::Direction::Outgoing).any(|n| !graph.orig_graph[n].visible()))
                .map(|(i, _)| Filter::ShowNeighbours(i, petgraph::Direction::Outgoing)).collect(),
            props.insts.clone().into_iter()
                .filter(|&(i, _)| graph.orig_graph.neighbors_directed(i.into(), petgraph::Direction::Incoming).any(|n| !graph.orig_graph[n].visible()))
                .map(|(i, _)| Filter::ShowNeighbours(i, petgraph::Direction::Incoming)).collect(),
            props.insts.clone().into_iter()
                // TODO: filter if all ancestors are visible
                .filter(|&(i, _)| graph.orig_graph.neighbors_directed(i.into(), petgraph::Direction::Incoming).any(|_| true))
                .map(|(i, _)| Filter::VisitSourceTree(i, true)).collect(),
            props.insts.clone().into_iter()
                .map(|(i, _)| Filter::VisitSourceTree(i, false)).collect(),
            props.insts.clone().into_iter()
                // TODO: filter if all successors are visible
                .filter(|&(i, _)| graph.orig_graph.neighbors_directed(i.into(), petgraph::Direction::Outgoing).any(|_| true))
                .map(|(i, _)| Filter::VisitSubTreeWithRoot(i, true)).collect(),
            props.insts.clone().into_iter()
                .map(|(i, _)| Filter::VisitSubTreeWithRoot(i, false)).collect(),
            props.insts.clone().into_iter()
                .map(|(_, q)| Filter::IgnoreQuantifier(q)).collect(),
            props.insts.clone().into_iter()
                .map(|(_, q)| Filter::IgnoreAllButQuantifier(q)).collect(),
            props.insts.clone().into_iter()
                .map(|(i, _)| Filter::ShowLongestPath(i)).collect(),
        ]
    } else {
        let mut mls = Vec::new();
        let mut mls_all = Vec::new();
        if props.found_mls.is_some_and(|mls| mls > 0) {
            mls = vec![Filter::SelectNthMatchingLoop(0)];
            mls_all = vec![Filter::ShowMatchingLoopSubgraph];
        };
        vec![
            vec![Filter::MaxNodeIdx(1000)],
            vec![Filter::IgnoreTheorySolving],
            vec![Filter::MaxInsts(DEFAULT_NODE_COUNT)],
            vec![Filter::MaxBranching(DEFAULT_NODE_COUNT)],
            vec![Filter::MaxDepth(6)],
            vec![Filter::ShowNamedQuantifier("name".to_string())],
            mls,
            mls_all,
        ]
    };
    let filters = filters.into_iter().map(|f| {
        if f.is_empty() {
            return html!{}
        }
        let icon = f[0].icon();
        let short_text = f[0].short_text().split(['|', '"']).enumerate().map(|(i, c)| {
            if i % 2 == 0 {
                c
            } else {
                "_"
            }
        }).collect::<String>();
        let long_text = f[0].long_text(false);

        let new_filter = props.new_filter.clone();
        let onlick = Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            for f in &f {
                new_filter.emit(f.clone());
            }
        });
        html! {
            <li><a href="#" draggable="false" title={long_text} onclick={onlick}>
                <div class="material-icons"><MatIcon>{icon}</MatIcon></div>
                {short_text}
            </a></li>
        }
    });
    html! {
    <>
        {for filters}
    </>
    }
}
