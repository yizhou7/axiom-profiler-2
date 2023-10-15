use yew::{prelude::*, virtual_dom::{VNode, VTag, VText}};
use scraper::{self, Selector};

#[derive(Properties, PartialEq)]
pub struct EdgesProps {
    pub svg_text: String,
}

#[function_component(Edges)]
pub fn edges(EdgesProps { svg_text }: &EdgesProps) -> Html {
    // let g_selector = Selector::parse("g > g").unwrap();
    let edge_selector = Selector::parse(".edge").unwrap();
    let svg = scraper::Html::parse_document(svg_text);
    let edge_tags = scraper::Html::select(&svg, &edge_selector);
    edge_tags.map(|edge| {
        let inner_html = edge.inner_html();
        let id: AttrValue = edge.value().id().unwrap().to_string().into();
        let class = classes!(edge.value().classes().map(String::from).collect::<Vec<String>>());
        html! {
        <Edge inner_html={inner_html} id={id} class={class}/>
        }
    }).collect()
}

#[derive(Properties, PartialEq)]
pub struct EdgeProps {
    pub inner_html: String,
    pub id: AttrValue,
    pub class: Classes,
}

#[function_component(Edge)]
fn edge(props: &EdgeProps) -> Html {
    let state = use_state(|| props.id.clone());
    let onclick = move |_| {
        log::debug!("edge selected!");
    };
    // extract path attributes 
    let path_selector = Selector::parse("path").unwrap();
    let svg = scraper::Html::parse_document(&props.inner_html);
    let mut path_tag = scraper::Html::select(&svg, &path_selector);
    let node = path_tag.next().unwrap().value();
    let fill = node.attr("fill").unwrap().to_string();
    let stroke = node.attr("stroke").unwrap().to_string();
    let d = node.attr("d").unwrap().to_string();
    // extract polygon attributes
    let polygon_selector = Selector::parse("polygon").unwrap();
    let mut polygon_tag = scraper::Html::select(&svg, &polygon_selector);
    let node = polygon_tag.next().unwrap().value();
    let polygon_fill = node.attr("fill").unwrap().to_string();
    let polygon_stroke = node.attr("stroke").unwrap().to_string();
    let points = node.attr("points").unwrap().to_string();
    html! {
        <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
            <path {fill} {stroke} {d} ></path>
            <polygon {polygon_fill} {polygon_stroke} {points} ></polygon>
        </g>
    }
}