use yew::{prelude::*, virtual_dom::{VNode, VTag, VText}};
use scraper::{self, Selector};
use crate::{svg_result::*, attribute_fetcher::*};

#[function_component(Nodes)]
pub fn nodes() -> Html {
    let node_selector = Selector::parse(".node").unwrap();
    let state = use_context::<GraphState>().expect("no ctx found");
    let svg_text = state.svg_text; 
    let svg = scraper::Html::parse_document(&svg_text);
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
    let svg = scraper::Html::parse_document(&props.inner_html);
    let EllipseAttr {fill, stroke, cx, cy, rx, ry} = EllipseAttr::get_attributes(&svg);
    // extract text attributes
    let TextAttr {text_anchor, x, y, font_family, font_size} = TextAttr::get_attributes(&svg);
    let text_selector = Selector::parse("text").unwrap();
    let inner_text: String = scraper::Html::select(&svg, &text_selector).flat_map(|el| el.text()).collect();
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
    let ctx = use_context::<GraphState>().expect("no ctx found");
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