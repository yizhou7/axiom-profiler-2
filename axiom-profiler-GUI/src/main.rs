use gloo_file::{callbacks::FileReader, File};
use scraper::{self, Selector};
use std::collections::HashMap;
use std::str::from_utf8;
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;
use prototype::parsers::{z3parser1, LogParser};
use viz_js::VizInstance;

mod svg_graph;
pub enum Msg {
    LoadedBytes(String, Vec<u8>),
    Files(Vec<File>),
}

pub struct FileDataComponent {
    files: Vec<String>,
    readers: HashMap<String, FileReader>,
}

impl Component for FileDataComponent {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            files: Vec::new(),
            readers: HashMap::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Files(files) => {
                log::info!("Files selected: {}", files.len());
                for file in files.into_iter() {
                    let file_name = file.name();
                    let task = {
                        let file_name = file_name.clone();
                        let link = ctx.link().clone();

                        gloo_file::callbacks::read_as_bytes(&file, move |res| {
                            link.send_message(Msg::LoadedBytes(
                                file_name,
                                res.expect("failed to read file"),
                            ))
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                true
            }
            Msg::LoadedBytes(file_name, data) => {
                log::info!("Processing: {}", file_name);

                let text_data = from_utf8(&data).unwrap().to_string(); //base64::encode(data);
                self.files.pop();
                self.files.push(text_data);
                self.readers.remove(&file_name);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change = ctx.link().callback(move |e: Event| {
            let mut selected_files = Vec::new();
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                let files = js_sys::try_iter(&files)
                    .unwrap()
                    .unwrap()
                    .map(|v| web_sys::File::from(v.unwrap()))
                    .map(File::from);
                selected_files.extend(files);
            }
            Msg::Files(selected_files)
        });
        html! {
            <div>
                <div>
                    {"Choose a log file:"}
                </div>
                <div>
                    <input type="file" accept=".log" onchange={on_change} multiple=false/>
                </div>
                <div>
                { for self.files.iter().map(|f| Self::view_file(f))}
                </div>
            </div>
        }
    }
}

impl FileDataComponent {
    fn view_file(data: &str) -> Html {
        log::debug!("Viewing file");
        html! {
            <div>
            <SVGResult trace_file_text={AttrValue::from(data.to_string())}/>
            </div>
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct SVGProps {
    pub trace_file_text: AttrValue,
}

#[function_component(SVGResult)]
fn svg_result(props: &SVGProps) -> Html {
    log::debug!("SVG result");
    let svg_text = use_state(|| html! {});
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
                                        // let fetched_svg: String = crate::svg_graph::create_svg_graph(&parser.get_sorted_dependencies());  
                    log::debug!("request done");
                    let length = fetched_svg.len();
                    log::debug!("new svg: {} bytes", length);
                    let svg = scraper::Html::parse_document(&fetched_svg);
                    let g_tags =
                        scraper::Html::select(&svg, &g_selector);
                    log::debug!("selected");
                    let _nodes = g_tags
                        .map(|node| {
                                let inner_html: AttrValue = 
                                node.inner_html().into();
                                let id: AttrValue = node.value().id().unwrap().to_string().into();
                                let class = classes!(node.value().classes().map(String::from).collect::<Vec<String>>());
                            html! {
                            <Node inner_html={inner_html} id={id} class={class}/>
                            }
                        });
                    let mut svg_tag = scraper::Html::select(&svg, &svg_selector);
                    let node = svg_tag.next().unwrap().value();
                    let _width = node.attr("width").unwrap().to_string();
                    let _height = node.attr("height").unwrap().to_string();
                    let _view_box = node.attr("viewBox").unwrap().to_string();
                    log::debug!("made nodes");
                    let svg_result = AttrValue::from(fetched_svg);
                    svg_text.set(Html::from_html_unchecked(svg_result));
                    log::debug!("set state");
                },
                   
            );
        })
    };

    html! {
        <>
        <button onclick={onclick}>{"Load file"}</button>
        <br/>
        // <textarea rows="50" cols="100" />
        {(*svg_text).clone()}
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
    };
    let inner = Html::from_html_unchecked(props.inner_html.clone().into());
    html! {
        <g id={props.id.clone()} class={props.class.clone()} onclick={onclick.clone()}>
        {inner}
        </g>
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
        <div>
            <h1>{"Yew testing"}</h1>
            <FileDataComponent/>
        </div>
        </>
    }
}

#[function_component(Test)]
fn test() -> Html {
    html! {
        <div>
        <h1>{"test"}</h1>
        </div>
    }
}

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[at("/")]
    App,
    #[at("/test")]
    Test,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::App => html! {
            <App/>
        },
        Route::Test => html! {
            <Test/>
        },
    }
}

#[function_component(Main)]
fn main() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("App is starting");
    yew::Renderer::<Main>::new().render();
}
