use yew::prelude::*;
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};
use crate::nodes::*;
use crate::edges::*;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| (String::new(), false));
    let onclick = {
        let text = props.trace_file_text.to_string();
        let svg_text = svg_text.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let svg_text = svg_text.clone();
            log::debug!("use effect");
            wasm_bindgen_futures::spawn_local(
                async move {
                    let mut parser = z3parser1::Z3Parser1::new();
                    parser.process_log(text);
                    let qi_graph = parser.get_instantiation_graph();
                    let dot_output = format!("{:?}", Dot::with_config(qi_graph, &[Config::EdgeNoLabel])); 
                    let graphviz = VizInstance::new().await;
                    let svg = graphviz
                        .render_svg_element(dot_output, viz_js::Options::default())
                        .expect("Could not render graphviz");
                    let fetched_svg = svg.outer_html(); 
                    svg_text.set((fetched_svg, true));
                },
                   
            );
        })
    };
    html! {
        <>
            <div>
            <button onclick={onclick}>{"Load file"}</button>
            </div>
            { if (*svg_text).1 {
                html! { <Graph svg_text={(*svg_text).0.clone()} /> }
            } else {
                html! {}
            }}
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct GraphProps {
    pub svg_text: String,
}

#[function_component(Graph)]
fn graph(props: &GraphProps) -> Html {
    // extract svg attributes
    let svg_selector = Selector::parse("svg").unwrap();
    let svg = scraper::Html::parse_document(&props.svg_text);
    let mut svg_tag = scraper::Html::select(&svg, &svg_selector);
    let node = svg_tag.next().unwrap().value();
    // let xmlns = node.attr("xmlns").unwrap().to_string();
    let width = node.attr("width").unwrap().to_string();
    let height = node.attr("height").unwrap().to_string();
    let view_box = node.attr("viewBox").unwrap().to_string();
    // extract top-level g attributes
    let g_selector = Selector::parse("g").unwrap();
    let g = scraper::Html::parse_document(&props.svg_text);
    let mut g_tag = scraper::Html::select(&g, &g_selector);
    let node = g_tag.next().unwrap().value();
    let id = node.attr("id").unwrap().to_string();
    let class = node.attr("class").unwrap().to_string();
    let transform = node.attr("transform").unwrap().to_string();
    // extract top-level polygon attributes
    let polygon_selector = Selector::parse("polygon").unwrap();
    let polygon = scraper::Html::parse_document(&props.svg_text);
    let mut polygon_tag = scraper::Html::select(&polygon, &polygon_selector);
    let node = polygon_tag.next().unwrap().value();
    let fill = node.attr("fill").unwrap().to_string();
    let stroke = node.attr("stroke").unwrap().to_string();
    let points = node.attr("points").unwrap().to_string();
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" {width} {height} viewBox={view_box}>
            <g {id} {class} {transform}>
                <polygon {fill} {stroke} {points}></polygon>
                <Nodes svg_text={(props.svg_text).clone()} />
                <Edges svg_text={(props.svg_text).clone()} />
            </g>
        </svg>
    }
}