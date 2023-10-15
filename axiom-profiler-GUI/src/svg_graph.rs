use yew::{prelude::*, virtual_dom::{VNode, VTag}};
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;
use petgraph::dot::{Dot, Config};

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| String::new());
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
                    svg_text.set(fetched_svg);
                },
                   
            );
        })
    };
    html! {
        <>
            <div>
            <button onclick={onclick}>{"Load file"}</button>
            </div>
            <svg xmlns="http://www.w3.org/2000/svg" width="206pt" height="116pt" viewBox="0.00 0.00 206.00 116.00">
                <g id="graph0" class="graph" transform="scale (1 1) rotate(0) translate(4 112)">
                    <polygon fill="white" stroke="none" points="-4,4 -4,-112 202,-112 202,4 -4,4"></polygon>
                    <NodeList svg_text={(*svg_text).clone()} />
                </g>
            </svg>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct NodeListProps {
    svg_text: String,
}

#[function_component(NodeList)]
fn node_list(NodeListProps { svg_text }: &NodeListProps) -> Html {
    let g_selector = Selector::parse("g > g").unwrap();
    let svg = scraper::Html::parse_document(svg_text);
    let g_tags = scraper::Html::select(&svg, &g_selector);
    g_tags.map(|node| {
        let html: AttrValue = node.html().into();
        // let inner_html: AttrValue = 
        // node.inner_html().into();
        // let id: AttrValue = node.value().id().unwrap().to_string().into();
        // let class = classes!(node.value().classes().map(String::from).collect::<Vec<String>>());
        // let inner_html = Html::from_html_unchecked(inner_html);
        let outer_html = Html::from_html_unchecked(html);
        html! {
        // <Node inner_html={inner_html} id={id} class={class}/>
        // <g id={id} class={class}>{inner_html}</g>
        // outer_html 
        <g id="node1" class="node">
            <title>{0}</title>
            <ellipse stroke="black" ry="18" fill="none" cy="-90" cx="27" rx="27"></ellipse>
            <text y="-85.8" x="27" font-size="14.00" text-anchor="middle" font-family="Times,serif">{722}</text>
        </g>
        }
    }).collect()
}

#[derive(Properties, PartialEq)]
pub struct NodeProps {
    pub inner_html: AttrValue,
    pub id: AttrValue,
    pub class: Classes,
}

#[function_component(Node)]
fn node(props: &NodeProps) -> Html {
    let state = use_state(|| props.id.clone());
    let onclick = move |_| {
        log::debug!("node selected!");
    };
    let inner = Html::from_html_unchecked(props.inner_html.clone().into());
    html! {
        <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
        {inner}
        </g>
    }
} 