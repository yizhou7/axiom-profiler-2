use yew::{prelude::*, virtual_dom::VNode};
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[derive(Default,Clone)]
struct SVGProperties {
    width: String,
    height: String,
    view_box: String,
    // xmlns: String,
    // xmlns_xlink: String,
}

#[function_component(SVGResult)]
pub fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| (html! {}, Vec::new(), SVGProperties::default()));
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
                    let parsed = Html::from_html_unchecked(AttrValue::from(fetched_svg));
                    let parsed_vlist = parsed.to_vlist_mut();
                    let g_tags =
                        scraper::Html::select(&svg, &g_selector);
                    log::debug!("selected");
                    let _nodes: Vec<VNode> = g_tags
                        .map(|node| {
                                let inner_html: AttrValue = 
                                node.inner_html().into();
                                // node.html().into();
                                let id: AttrValue = node.value().id().unwrap().to_string().into();
                                let class = classes!(node.value().classes().map(String::from).collect::<Vec<String>>());
                            html! {
                            <Node inner_html={inner_html} id={id} class={class}/>
                            }
                        }).collect();
                    let mut svg_tag = scraper::Html::select(&svg, &svg_selector);
                    let node = svg_tag.next().unwrap().value();
                    let _width = node.attr("width").unwrap().to_string();
                    let _height = node.attr("height").unwrap().to_string();
                    let _view_box = node.attr("viewBox").unwrap().to_string();
                    // let _xmlns = node.attr("xmlns").unwrap().to_string();
                    // let _xmlns_xlink = node.attr("xmlns:xlink").unwrap().to_string();
                    let svg_properties = SVGProperties {
                        width: _width,
                        height: _height,
                        view_box: _view_box,
                        // xmlns: _xmlns,
                        // xmlns_xlink: _xmlns_xlink,
                    }; 
                    log::debug!("made nodes");
                    let svg_result = AttrValue::from(fetched_svg);
                    svg_text.set((
                        Html::from_html_unchecked(svg_result),
                        _nodes,
                        svg_properties));
                    log::debug!("set state");
                },
                   
            );
        })
    };

    let svg_properties = (*svg_text).2.clone();
    html! {
        <>
        <button onclick={onclick}>{"Load file"}</button>
        <br/>
        // <textarea rows="50" cols="100" />
        {(*svg_text).0.clone()}
        // <svg width={svg_properties.width} height={svg_properties.height} viewBox={svg_properties.view_box} xmlns="http://www.w3.org/2000/svg">
        <svg width="206" height="116" viewBox="0 0 206 116" xmlns="http://www.w3.org/2000/svg" xlink="http://www.w3.org/1999/xlink">
            <g id="graph0" class="graph" transform="scale(1 1) rotate(0) translate(4 112)">
            <polygon fill="white" stroke="none" points="-4,4 -4,-112 202,-112 202,4 -4,4"></polygon>
            { for (*svg_text).1.clone()}
            </g>
        </svg>
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
    let state = use_state(|| props.id.clone());
    let onclick = move |_| {
        log::debug!("{}", *state);
        log::debug!("node selected!");
    };
    let inner = Html::from_html_unchecked(props.inner_html.clone().into());
    html! {
        <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
        {inner}
        </g>
    }
} 