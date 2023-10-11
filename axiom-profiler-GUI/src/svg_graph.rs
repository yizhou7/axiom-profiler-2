use yew::{prelude::*, virtual_dom::{VNode, VTag}};
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| (html! {}, Vec::new()));
    let g_selector = Selector::parse("g > g").unwrap();
    let svg_selector = Selector::parse("svg").unwrap();
    let onclick = {
        let text = props.trace_file_text.to_string();
        let svg_text = svg_text.clone();
        let g_selector = g_selector.clone();
        let svg_selector = svg_selector.clone();
        Callback::from(move |_| {
            let text = text.to_string();
            let svg_text = svg_text.clone();
            let g_selector = g_selector.clone();
            let svg_selector = svg_selector.clone();
            log::debug!("use effect");
            wasm_bindgen_futures::spawn_local(
                async move {
                    let mut parser = z3parser1::Z3Parser1::new();
                    parser.process_log(text);
                    let dot_output = parser.get_dot_output_as_string();
                    let graphviz = VizInstance::new().await;
                    let svg = graphviz
                        .render_svg_element(dot_output, viz_js::Options::default())
                        .expect("Could not render graphviz");
                    let fetched_svg = svg.outer_html(); 
                    log::debug!("request done");
                    let length = fetched_svg.len();
                    log::debug!("new svg: {} bytes", length);
                    let svg = scraper::Html::parse_document(&fetched_svg);
                    let g_tags =
                        scraper::Html::select(&svg, &g_selector);
                    log::debug!("selected");
                    let _nodes: Vec<VNode> = g_tags
                        .map(|node| {
                                let inner_html: AttrValue = 
                                node.inner_html().into();
                                // node.html().into();
                                let id_str = node.value().id().unwrap().to_string();
                                let id : AttrValue = id_str.clone().into();
                                let class = classes!(node.value().classes().map(String::from).collect::<Vec<String>>());
                            html! {
                                {if id_str.contains("a_") {
                                    html! {}
                                } else {
                                    html! { <Node inner_html={inner_html} id={id} class={class}/> }
                                }}
                            }
                        }).collect();
                    let mut svg_tag = scraper::Html::select(&svg, &svg_selector);
                    let node = svg_tag.next().unwrap().value();
                    let _width = node.attr("width").unwrap().to_string();
                    let _height = node.attr("height").unwrap().to_string();
                    let _view_box = node.attr("viewBox").unwrap().to_string();
                    log::debug!("made nodes");
                    let svg_result = AttrValue::from(fetched_svg);
                    svg_text.set((Html::from_html_unchecked(svg_result),_nodes));
                    log::debug!("set state");
                },
                   
            );
        })
    };

    let mut polygon = VTag::new("polygon");
    polygon.add_attribute("fill", "white");
    polygon.add_attribute("stroke", "none");
    polygon.add_attribute("points", "-4,4 -4,-112 202,-112 202,4 -4,4");

    let mut group = VTag::new("g"); 
    group.add_attribute("id","graph0");
    group.add_attribute("class","graph");
    group.add_attribute("transform","scale(1 1) rotate(0) translate(4 112)");
    group.add_child(VNode::from(polygon));
    group.add_children((*svg_text).1.clone().into_iter());

    let mut svg_graph = VTag::new("svg");
    svg_graph.add_attribute("xmlns", "http://www.w3.org/2000/svg");
    svg_graph.add_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    svg_graph.add_attribute("width", "206pt");
    svg_graph.add_attribute("height", "116pt");
    svg_graph.add_attribute("viewBox", "0.00 0.00 206.00 116.00");
    svg_graph.add_child(VNode::from(group));

    html! {
        <>
        <button onclick={onclick}>{"Load file"}</button>
        <br/>
        {(*svg_text).0.clone()}
        <div>
            { for (*svg_text).1.clone()}
        </div>
        {VNode::from(svg_graph)}
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub inner_html: AttrValue,
    pub id: AttrValue,
    pub class: Classes,
}

#[function_component(Node)]
fn node(props: &NodeProps) -> Html {
    let state = use_state(|| (props.id.clone(), false));
    let onclick = move |_| {
        log::debug!("node selected!");
    };
    let inner = Html::from_html_unchecked(props.inner_html.clone().into());
    html! {
        {if (*state).clone().1 == true {
            html! {
                <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
                {inner}
                </g>
            }
        } else {
            html! {}
        }}

    }
} 