use yew::prelude::*;
use scraper::{self, Selector};

use crate::svg_result::GraphCtxt;

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
    // let _ = use_state(|| None);
    // let onclick = {
    //     move |_| {
    //         log::debug!("edge selected!");
    //     }
    // };
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
    let title_selector = Selector::parse("title").unwrap();
    let title_text: String = scraper::Html::select(&svg, &title_selector).flat_map(|el| el.text()).collect();
    let ctx = use_context::<GraphCtxt>().expect("no ctx found");
    let visibility = match parse_input(&title_text) {
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
        // <g id={props.id.clone()} class={props.class.clone()} {visibility} onclick={onclick.clone()}>
        <g id={props.id.clone()} class={props.class.clone()} {visibility} >
            <path {fill} {stroke} {d} ></path>
            <polygon {polygon_fill} {polygon_stroke} {points} ></polygon>
        </g>
    }
}

fn parse_input(input: &str) -> Option<(usize, usize)> {
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