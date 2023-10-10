use yew::{prelude::*, virtual_dom::VNode};
use scraper::{self, Selector};
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;

enum Msg {
    SvgGraph(VNode)
}

#[derive(Properties, PartialEq, Clone, Eq)]
pub struct SVGProps {
    pub trace_file_text: String,
}

pub struct SVGResult {
    log_file: String,
}

impl Component for SVGResult {
    type Message = Msg;
    type Properties = SVGProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            log_file: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onclick = ctx.link().callback(move |_| {
            let mut svg_text; 
            wasm_bindgen_futures::spawn_local(
            async move {
                let mut parser = z3parser1::Z3Parser1::new();
                let text = ctx.props().trace_file_text;
                parser.process_log(text);
                let dot_output = parser.get_dot_output_as_string();
                let graphviz = VizInstance::new().await;
                let svg = graphviz
                    .render_svg_element(dot_output, viz_js::Options::default())
                    .expect("Could not render graphviz");
                let fetched_svg = svg.outer_html(); 
                let svg_result = AttrValue::from(fetched_svg);
                svg_text = Html::from_html_unchecked(svg_result);
            },);
            Msg::SvgGraph(svg_text)
        });  
        html! {
            <>
            <button onclick={onclick}>{"Render instantiation graph"}</button>
            <br/>
            // <textarea rows="50" cols="100" />
            {ctx.props().trace_file_text.clone()}
            </>
        }
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
    };
    let inner = Html::from_html_unchecked(props.inner_html.clone().into());
    html! {
        <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
        {inner}
        </g>
    }
}