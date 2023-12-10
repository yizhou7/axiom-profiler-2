use gloo::console::log;
use web_sys::HtmlElement;
use yew::{prelude::*, virtual_dom::VNode};
use indexmap::map::IndexMap;
use petgraph::graph::NodeIndex;
use smt_log_parser::{parsers::z3::inst_graph::{InstInfo, EdgeInfo}, items::DepType};
use smt_log_parser::parsers::z3::inst_graph::EdgeType;
use material_yew::WeakComponentLink;


pub struct InstsInfo {
    is_expanded: IndexMap<usize, bool>,
    selected_nodes_ref: NodeRef,
}

pub enum Msg {
    AddNode(NodeIndex),
    RemoveNode(NodeIndex),
    RemoveAll,
    ToggleOpen(NodeIndex),
}

#[derive(Properties, PartialEq)]
pub struct InstsInfoProps {
    pub selected_nodes: Vec<InstInfo>,
    pub selected_edges: Vec<(NodeIndex, NodeIndex, EdgeInfo)>,
    pub last_selected_node: Option<NodeIndex>,
    pub weak_link: WeakComponentLink<InstsInfo>,
}

impl Component for InstsInfo {
    type Message = Msg;

    type Properties = InstsInfoProps;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.props()
            .weak_link
            .borrow_mut()
            .replace(ctx.link().clone());
        Self {
            is_expanded: IndexMap::new(),
            selected_nodes_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddNode(node) => {
                // When adding a single new node to the vector,
                // close all  
                for val in self.is_expanded.values_mut() {
                    *val = false;
                }
                self.is_expanded.insert(node.index(), true);
                log!("Adding node {}", node.index());
                true
            },
            Msg::RemoveNode(node) => {
                self.is_expanded.remove(&node.index());
                log!("Removing node {}", node.index());
                true
            },
            Msg::RemoveAll => {
                self.is_expanded.clear();
                true
            },
            Msg::ToggleOpen(node) => {
                let open_value = self.is_expanded.get_mut(&node.index()).unwrap();
                *open_value = !*open_value;
                log!(format!("Setting the open-value of {} to {}", node.index(), *open_value));
                false
            }
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        let selected_nodes_details = self.selected_nodes_ref.cast::<HtmlElement>().expect("not attached to div element");
        let node_details = selected_nodes_details.get_elements_by_tag_name("details");
        for i in 0..node_details.length() {
            let node_detail = node_details.item(i).unwrap();
            let node_id = node_detail.id().parse::<usize>().unwrap();
            if *self.is_expanded.get(&node_id).unwrap() {
                let _ = node_detail.set_attribute("open", "true");
            } else {
                let _ = node_detail.remove_attribute("open");
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let selected_edges_info: Vec<VNode> = ctx.props()
            .selected_edges
            .iter()
            .map(|(from, to, data)| {
                html! {
                <details>
                    <summary>{format!("Dependency from {} to {}", from.index(), to.index())}</summary>
                    {match data.edge_data.edge_type {
                        EdgeType::Direct(DepType::Term) => html! {
                            <div>
                            <h4>{"Blame term: "}</h4><p>{data.blame_term.clone()}</p>
                            </div>
                        }, 
                        EdgeType::Direct(DepType::Equality) => html! {
                            <div>
                            <h4>{"Equality: "}</h4><p>{data.blame_term.clone()}</p>
                            </div>
                        },
                        _ => html! {} 
                    }}
                </details>
                }
            })
            .collect();
        let on_click = {
            let link = ctx.link().clone();
            Callback::from(move |node: NodeIndex| {
                link.send_message(Msg::ToggleOpen(node))
            })
        };
        html! {
            <>
                <h2>{"Information about selected nodes:"}</h2>
                <div ref={self.selected_nodes_ref.clone()}><SelectedNodesInfo selected_nodes={ctx.props().selected_nodes.clone()} {on_click} /></div>
                <h2>{"Information about selected dependencies:"}</h2>
                { for selected_edges_info }
            </>
        }
    }
}

#[derive(Properties, PartialEq)]
struct SelectedNodesInfoProps {
    selected_nodes: Vec<InstInfo>,
    on_click: Callback<NodeIndex>,
}

#[function_component(SelectedNodesInfo)]
fn selected_nodes_info(SelectedNodesInfoProps { selected_nodes, on_click }: &SelectedNodesInfoProps) -> Html {
    selected_nodes 
            .iter()
            .map(|selected_inst| { 
                let get_ul = |label: &str, items: &Vec<String>| html! {
                    <>
                        <h4>{label}</h4>
                        <ul>{for items.iter().map(|item| html!{<li>{item}</li>})}</ul>
                    </>
                };
                let on_select = {
                    let on_click = on_click.clone();
                    let selected_inst = selected_inst.clone();
                    Callback::from(move |_| {
                        on_click.emit(selected_inst.node_index.clone())
                    })
                };
                // let open = *self.is_expanded.get(&selected_inst.node_index.index()).unwrap(); 
                html! {
                <details id={format!("{}", selected_inst.node_index.index())} onclick={on_select}>
                    <summary>{format!("Node {}", selected_inst.node_index.index())}</summary>
                    <ul>
                        <li><h4>{"Instantiation happens at line number: "}</h4><p>{if let Some(val) = selected_inst.line_no {format!("{val}")} else { String::new() }}</p></li>
                        <li><h4>{"Cost: "}</h4><p>{selected_inst.cost}</p></li>
                        <li><h4>{"Instantiated formula: "}</h4><p>{&selected_inst.formula}</p></li>
                        <li>{get_ul("Blamed terms: ", &selected_inst.blamed_terms)}</li>
                        <li>{get_ul("Bound terms: ", &selected_inst.bound_terms)}</li>
                        <li>{get_ul("Yield terms: ", &selected_inst.yields_terms)}</li>
                        <li>{get_ul("Equality explanations: ", &selected_inst.equality_expls)}</li>
                        <li><h4>{"Resulting term: "}</h4><p>{if let Some(ref val) = selected_inst.resulting_term {format!("{val}")} else { String::new() }}</p></li>
                    </ul>
                </details>
            }})
            .collect()
}