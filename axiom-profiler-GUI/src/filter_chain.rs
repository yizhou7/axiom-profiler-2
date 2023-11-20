use yew::prelude::*;
use crate::graph_filters::{GraphFilter, Filter};

pub enum Msg {
    AddFilter(Filter),
    RemoveNthFilter(usize),
    ResetFilters,
}

pub struct FilterChain {
    filter_chain: Vec<Filter>,
}

#[derive(Properties, PartialEq)]
pub struct FilterChainProps {
    pub apply_filter: Callback<Filter>,
    pub reset_graph: Callback<()>,
    pub render_graph: Callback<()>,
    pub dependency: AttrValue,
}

impl Component for FilterChain {
    type Message = Msg;
    type Properties = FilterChainProps;

    fn create(ctx: &Context<Self>) -> Self {
        let filter_chain = vec![Filter::IgnoreTheorySolving, Filter::MaxInsts(125)];
        for &filter in &filter_chain {
            ctx.props().apply_filter.emit(filter);
        }
        ctx.props().render_graph.emit(());
        Self {
            filter_chain,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddFilter(filter) => {
                self.filter_chain.push(filter);
                ctx.props().apply_filter.emit(filter);
                ctx.props().render_graph.emit(());
                true
            },
            Msg::RemoveNthFilter(n) => {
                self.filter_chain.remove(n);
                ctx.props().reset_graph.emit(());
                for &filter in &self.filter_chain {
                    ctx.props().apply_filter.emit(filter);
                }
                ctx.props().render_graph.emit(());
                true
            },
            Msg::ResetFilters => {
                self.filter_chain = vec![Filter::IgnoreTheorySolving, Filter::MaxInsts(125)];
                ctx.props().reset_graph.emit(());
                for &filter in &self.filter_chain {
                    ctx.props().apply_filter.emit(filter);
                }
                ctx.props().render_graph.emit(());
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
        let reset_filters = ctx.link().callback(|_| Msg::ResetFilters);

        let add_filter = ctx.link().callback(Msg::AddFilter);
        html!(
            <div>
                <GraphFilter
                    add_filter={add_filter.clone()}
                    dependency={ctx.props().dependency.clone()}
                />
                <h2>{"Filter chain:"}</h2>
                {for filter_chain}
                <button onclick={reset_filters}>{"Reset to default"}</button>
            </div>
        )
    }
}