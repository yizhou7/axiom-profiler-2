use yew::{prelude::*, virtual_dom::{VNode, VTag, VText}};
use scraper::{self, Selector};
use crate::svg_graph::*;

#[derive(Properties, PartialEq)]
pub struct NodesProps {
    pub svg_text: String,
}

#[function_component(Nodes)]
pub fn nodes(NodesProps { svg_text }: &NodesProps) -> Html {
    // let g_selector = Selector::parse("g > g").unwrap();
    let node_selector = Selector::parse(".node").unwrap();
    let svg = scraper::Html::parse_document(svg_text);
    let node_tags = scraper::Html::select(&svg, &node_selector);
    node_tags.map(|node| {
        let inner_html = node.inner_html();
        let id: AttrValue = node.value().id().unwrap().to_string().into();
        let class = classes!(node.value().classes().map(String::from).collect::<Vec<String>>());
        html! {
        <Node inner_html={inner_html} id={id} class={class}/>
        }
    }).collect()
}

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub inner_html: String,
    pub id: AttrValue,
    pub class: Classes,
}

#[function_component(Node)]
fn node(props: &NodeProps) -> Html {
    let _ = use_state(|| props.id.clone());
    let onclick = move |_| {
        log::debug!("node selected!");
    };
    // extract ellipse attributes 
    let ellipse_selector = Selector::parse("ellipse").unwrap();
    let svg = scraper::Html::parse_document(&props.inner_html);
    let mut ellipse_tag = scraper::Html::select(&svg, &ellipse_selector);
    let node = ellipse_tag.next().unwrap().value();
    let fill = node.attr("fill").unwrap().to_string();
    let stroke = node.attr("stroke").unwrap().to_string();
    let cx = node.attr("cx").unwrap().to_string();
    let cy = node.attr("cy").unwrap().to_string();
    let rx = node.attr("rx").unwrap().to_string();
    let ry = node.attr("ry").unwrap().to_string();
    // extract text attributes
    let text_selector = Selector::parse("text").unwrap();
    let mut text_tag = scraper::Html::select(&svg, &text_selector);
    let node = text_tag.next().unwrap().value();
    let inner_text: String = scraper::Html::select(&svg, &text_selector).flat_map(|el| el.text()).collect();
    let text_anchor = node.attr("text-anchor").unwrap().to_string();
    let x = node.attr("x").unwrap().to_string();
    let y = node.attr("y").unwrap().to_string();
    let font_family = node.attr("font-family").unwrap().to_string();
    let font_size = node.attr("font-size").unwrap().to_string();
    let mut text_node = VTag::new("text");
    text_node.add_attribute("text-anchor", text_anchor);
    text_node.add_attribute("x", x);
    text_node.add_attribute("y", y);
    text_node.add_attribute("font-family", font_family);
    text_node.add_attribute("font-size", font_size);
    // Set the text content
    let inner_text_node: VNode = VText::new(inner_text.clone()).into(); 
    text_node.add_child(inner_text_node);
    // Convert the VTag to a VNode
    let text_vnode: VNode = text_node.into();
    // Decide whether this node should be visible or not based on max line nr.
    let ctx = use_context::<MaxLineNr>().expect("no ctx found");
    let max_line_nr = ctx.max_line_nr;
    let visibility = match inner_text.parse::<i32>() {
        Ok(line_nr) => if line_nr > max_line_nr { "hidden" } else { "visible" },
        Err(_) => "visible", 
    };
    html! {
        <g id={props.id.clone()} class={props.class.clone()} {visibility} onclick={onclick.clone()}>
            <ellipse {fill} {stroke} {cx} {cy} {rx} {ry} ></ellipse>
            { text_vnode }
        </g>
    }
}