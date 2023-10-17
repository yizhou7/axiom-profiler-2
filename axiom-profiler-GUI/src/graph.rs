use yew::prelude::*;
use scraper::{self, Selector};
use crate::nodes::*;
use crate::edges::*;
use crate::svg_result::GraphState;

#[function_component(Graph)]
pub fn graph() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    // extract svg attributes
    let svg_selector = Selector::parse("svg").unwrap();
    let svg = scraper::Html::parse_document(&state.svg_text);
    let mut svg_tag = scraper::Html::select(&svg, &svg_selector);
    let node = svg_tag.next().unwrap().value();
    // let xmlns = node.attr("xmlns").unwrap().to_string();
    let width = node.attr("width").unwrap().to_string();
    let height = node.attr("height").unwrap().to_string();
    let view_box = node.attr("viewBox").unwrap().to_string();
    // extract top-level g attributes
    let g_selector = Selector::parse("g").unwrap();
    let g = scraper::Html::parse_document(&state.svg_text);
    let mut g_tag = scraper::Html::select(&g, &g_selector);
    let node = g_tag.next().unwrap().value();
    let id = node.attr("id").unwrap().to_string();
    let class = node.attr("class").unwrap().to_string();
    let transform = node.attr("transform").unwrap().to_string();
    // extract top-level polygon attributes
    let polygon_selector = Selector::parse("polygon").unwrap();
    let polygon = scraper::Html::parse_document(&state.svg_text);
    let mut polygon_tag = scraper::Html::select(&polygon, &polygon_selector);
    let node = polygon_tag.next().unwrap().value();
    let fill = node.attr("fill").unwrap().to_string();
    let stroke = node.attr("stroke").unwrap().to_string();
    let points = node.attr("points").unwrap().to_string();
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" {width} {height} viewBox={view_box}>
            <g {id} {class} {transform}>
                <polygon {fill} {stroke} {points}></polygon>
                <Nodes />
                <Edges />
            </g>
        </svg>
    }
}