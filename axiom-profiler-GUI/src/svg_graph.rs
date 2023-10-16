use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::HtmlInputElement;
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
    let svg_text = use_state(|| None);
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
                    svg_text.set(Some(fetched_svg));
                },
                   
            );
        })
    };
    html! {
        <>
            <div>
                <button onclick={onclick}>{"Render instantiation graph"}</button>
            </div>
            { if let Some(text) = (*svg_text).clone() {
                html! { <Graph svg_text={text} /> }
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

#[derive(Clone, Debug, PartialEq)]
pub struct MaxLineNr {
    pub max_line_nr: i32,
}

#[function_component(Graph)]
fn graph(props: &GraphProps) -> Html {
    let max_line_nr = use_state(|| i32::MAX);
    let oninput = Callback::from({
        let max_line_nr = max_line_nr.clone();
        move |input_event: InputEvent| {
            let target: HtmlInputElement = input_event
                .target()
                .unwrap_throw()
                .dyn_into()
                .unwrap_throw();
            match target.value().to_string().parse::<i32>() {
                Ok(line_nr) => max_line_nr.set(line_nr),
                Err(_) => max_line_nr.set(i32::MAX),
            }
        }
    });
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
        <>
            <div>
                <input type="number" {oninput} />
            </div>
            <ContextProvider<MaxLineNr> context={MaxLineNr{max_line_nr: *max_line_nr}}>
                <svg xmlns="http://www.w3.org/2000/svg" {width} {height} viewBox={view_box}>
                    <g {id} {class} {transform}>
                        <polygon {fill} {stroke} {points}></polygon>
                        <Nodes svg_text={(props.svg_text).clone()} />
                        <Edges svg_text={(props.svg_text).clone()} />
                    </g>
                </svg>
            </ContextProvider<MaxLineNr>>
        </>
    }
}