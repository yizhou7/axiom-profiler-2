use yew::prelude::*;
use std::fmt::Display;
use crate::graph_filter::GraphFilter;

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

pub enum Msg {
    AddFilter(Filter),
    RemoveNthFilter(usize),
    RemoveAllFilters,
}

pub struct FilterChain {
    filter_chain: Vec<Filter>,
}

#[derive(Properties, PartialEq)]
pub struct FilterChainProps {
    pub apply_filter: Callback<Filter>,
    pub reset_graph: Callback<()>,
    pub dependency: AttrValue,
}

impl Component for FilterChain {
    type Message = Msg;
    type Properties = FilterChainProps;

    fn create(ctx: &Context<Self>) -> Self {
        let filter_chain = vec![Filter::IgnoreTheorySolving, Filter::MaxInsts(250)];
        for &filter in &filter_chain {
            ctx.props().apply_filter.emit(filter);
        }
        Self {
            filter_chain,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddFilter(filter) => {
                self.filter_chain.push(filter);
                // TODO: apply filter that was just added to currently displayed graph 
                // and compute how many nodes the current graph has such that user can
                // decide to render it or not
                ctx.props().apply_filter.emit(filter);
                true
            },
            Msg::RemoveNthFilter(n) => {
                self.filter_chain.remove(n);
                ctx.props().reset_graph.emit(());
                for &filter in &self.filter_chain {
                    ctx.props().apply_filter.emit(filter);
                }
                true
            },
            Msg::RemoveAllFilters => {
                self.filter_chain = Vec::new();
                ctx.props().reset_graph.emit(());
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
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
        let remove_all_filters = ctx.link().callback(|_| Msg::RemoveAllFilters);

        let add_filter = ctx.link().callback(Msg::AddFilter);
        html!(
            <div>
                <GraphFilter
                    add_filter={add_filter.clone()}
                    dependency={ctx.props().dependency.clone()}
                />
                <h3>{"Filter chain:"}</h3>
                {for filter_chain}
                <button onclick={remove_all_filters}>{"Remove all filters"}</button>
            </div>
        )
    }
}