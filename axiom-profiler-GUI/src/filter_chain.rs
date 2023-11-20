use yew::prelude::*;
use crate::graph_filters::{GraphFilter, Filter};
// use gloo_console::log;
use web_sys::window;

pub enum Msg {
    AddFilter(Filter),
    RemoveNthFilter(usize),
    ResetFilters,
    GetUserPermission,
    // CautiouslyRender,
}

pub struct FilterChain {
    filter_chain: Vec<Filter>,
    prev_filter_chain: Vec<Filter>,
}

#[derive(Properties, PartialEq)]
pub struct FilterChainProps {
    pub apply_filter: Callback<Filter>,
    pub reset_graph: Callback<()>,
    pub render_graph: Callback<()>,
    pub get_explicit_permission: bool,
    pub explicit_render: Callback<bool>,
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
            prev_filter_chain: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddFilter(filter) => {
                self.prev_filter_chain = self.filter_chain.clone();
                self.filter_chain.push(filter);
                ctx.props().apply_filter.emit(filter);
                ctx.props().render_graph.emit(());
                // ctx.link().send_message(Msg::CautiouslyRender);
                true
            },
            Msg::RemoveNthFilter(n) => {
                self.prev_filter_chain = self.filter_chain.clone();
                self.filter_chain.remove(n);
                ctx.props().reset_graph.emit(());
                for &filter in &self.filter_chain {
                    ctx.props().apply_filter.emit(filter);
                }
                ctx.props().render_graph.emit(());
                // ctx.link().send_message(Msg::CautiouslyRender);
                true
            },
            Msg::ResetFilters => {
                self.prev_filter_chain = self.filter_chain.clone();
                self.filter_chain = vec![Filter::IgnoreTheorySolving, Filter::MaxInsts(125)];
                ctx.props().reset_graph.emit(());
                for &filter in &self.filter_chain {
                    ctx.props().apply_filter.emit(filter);
                }
                ctx.props().render_graph.emit(());
                // ctx.link().send_message(Msg::CautiouslyRender);
                true 
            },
            Msg::GetUserPermission => {
                if let Some(window) = window() {
                    // Show the dialog with custom content
                    let result = window.confirm_with_message("Warning: The current graph contains a large number of nodes, rendering might be slow. Do you want to proceed?");
                    match result {
                        Ok(true) => {
                            // if the user wishes to render the current graph, we do so
                            ctx.props().explicit_render.emit(true);
                            true
                        }
                        Ok(false) => {
                            // if user does not wish to render the current graph, we set the 
                            // filter chain to the previous filter chain, apply all those filters
                            // and render the graph again
                            ctx.props().explicit_render.emit(false);
                            self.filter_chain = self.prev_filter_chain.clone();
                            for &filter in &self.filter_chain {
                                ctx.props().apply_filter.emit(filter);
                            }
                            ctx.props().render_graph.emit(());
                            false
                        }
                        Err(_) => {
                            // Handle the case where an error occurred
                            false
                        }
                    }
                } else {
                    false
                } 
            }
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
        if ctx.props().get_explicit_permission {
            ctx.link().send_message(Msg::GetUserPermission);
        }

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