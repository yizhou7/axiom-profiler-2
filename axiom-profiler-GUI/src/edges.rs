use yew::prelude::*;
use scraper::{self, Selector};

use crate::{svg_result::GraphState, attribute_fetcher::{PathAttr, TagAttributes, PolygonAttr}};

#[function_component(Edges)]
pub fn edges() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    let svg_text = state.svg_text; 
    let edge_selector = Selector::parse(".edge").unwrap();
    let svg = scraper::Html::parse_document(&svg_text);
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
    let svg = scraper::Html::parse_document(&props.inner_html);
    let PathAttr{fill, stroke, d} = PathAttr::get_attributes(&svg);
    let PolygonAttr{fill: polygon_fill, stroke: polygon_stroke, points} = PolygonAttr::get_attributes(&svg);
    let title_selector = Selector::parse("title").unwrap();
    let title_text: String = scraper::Html::select(&svg, &title_selector).flat_map(|el| el.text()).collect();
    let ctx = use_context::<GraphState>().expect("no ctx found");
    let visibility = match parse_edge_title(&title_text) {
        Some((from_node_idx, to_node_idx)) => {
            match (ctx.line_nr_of_node.get(&from_node_idx), ctx.line_nr_of_node.get(&to_node_idx)) {
                (Some(&from_node_line_nr), Some(&to_node_line_nr)) => {
                    if from_node_line_nr > ctx.max_line_nr as usize || to_node_line_nr > ctx.max_line_nr as usize {
                        log::debug!("hidden branch");
                        "hidden"
                    } else {
                        log::debug!("first visible branch");
                        "visible"
                    }
                },
                (_, _) => {log::debug!("second visible branch"); "visible"}
            }
        }, 
        _ => {log::debug!("third visible branch"); "visible"}
    };
    html! {
        <g id={props.id.clone()} class={props.class.clone()} {visibility} >
            <path {fill} {stroke} {d} ></path>
            <polygon {polygon_fill} {polygon_stroke} {points} ></polygon>
        </g>
    }
}

fn parse_edge_title(input: &str) -> Option<(usize, usize)> {
    // Split the input string by "->" and parse the integers
    let parts: Vec<&str> = input.trim().split("->").collect();

    if parts.len() == 2 {
        let x = parts[0].trim().parse::<i32>();
        let y = parts[1].trim().parse::<i32>();

        // Check if parsing was successful
        match (x, y) {
            (Ok(x_val), Ok(y_val)) => Some((x_val as usize, y_val as usize)),
            _ => None,
        }
    } else {
        None
    }
}