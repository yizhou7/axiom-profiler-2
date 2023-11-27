use gloo_file::{callbacks::FileReader, FileList};
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use smt_log_parser::parsers::{AsyncBufferRead, AsyncCursorRead, AsyncParser, LogParser};
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::svg_result::SVGResult;

mod filter_chain;
mod graph;
mod graph_filters;
mod input_state;
mod selected_node;
mod svg_result;
mod toggle_switch;
mod graph_container;
// mod select_dropdown;
pub enum Msg {
    LoadedBytes(String, Vec<u8>),
    Files(Option<FileList>),
}

pub struct FileDataComponent {
    files: Vec<String>,
    parsers: Vec<ParserData>,
    readers: Vec<FileReader>,
}

impl Component for FileDataComponent {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            files: Vec::new(),
            parsers: Vec::new(),
            readers: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Files(files) => {
                let Some(files) = files else {
                    return false;
                };
                let mut changed = !self.files.is_empty() || !self.readers.is_empty();
                self.files.clear();
                self.readers.clear();
                log::info!("Files selected: {}", files.len());
                for file in files.into_iter() {
                    let file_name = file.name();
                    if true {
                        // Old reader where all files are loaded as strings
                        let task = {
                            let link = ctx.link().clone();
                            gloo_file::callbacks::read_as_bytes(&file, move |res| {
                                link.send_message(Msg::LoadedBytes(
                                    file_name,
                                    res.expect("failed to read file"),
                                ))
                            })
                        };
                        self.readers.push(task);
                    } else {
                        // Turn into stream
                        let blob: &web_sys::Blob = file.as_ref();
                        let stream = ReadableStream::from_raw(blob.stream().unchecked_into());
                        match stream.try_into_async_read() {
                            Ok(stream) => {
                                let parser = Z3Parser::from_async(stream.buffer());
                                self.parsers.push(ParserData::new(parser));
                                changed = true;
                            }
                            Err((_err, _stream)) => {
                                let link = ctx.link().clone();
                                let reader =
                                    gloo_file::callbacks::read_as_bytes(file, move |res| {
                                        link.send_message(Msg::LoadedBytes(
                                            file_name,
                                            res.expect("failed to read file"),
                                        ))
                                    });
                                self.readers.push(reader);
                            }
                        };
                    }
                }
                changed
            }
            Msg::LoadedBytes(file_name, data) => {
                log::info!("Processing: {}", file_name);
                if true {
                    // Old reader where all files are loaded as strings
                    let text_data = String::from_utf8(data).unwrap();
                    self.files.push(text_data);
                } else {
                    let parser = Z3Parser::from_async(data.into_async_cursor());
                    self.parsers.push(ParserData::new(parser));
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change = ctx.link().callback(move |e: Event| {
            let files = e.target_dyn_into::<HtmlInputElement>().unwrap().files();
            Msg::Files(files.map(FileList::from))
        });
        html! {
            <div>
                <div style="height: 15vh;">
                    <h1>{"Axiom Profiler"}</h1>
                    <h2>{"Choose a log file:"}</h2>
                    <input type="file" accept=".log" onchange={on_change} multiple=false/>
                </div>
                <div style="display: flex; ">
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
            <SVGResult trace_file_text={AttrValue::from(data.to_string())}/>
        }
    }
}

pub struct ParserData {
    parser: AsyncParser<'static, Z3Parser>,
    parsed: Option<Z3Parser>,
}
impl ParserData {
    pub fn new(parser: AsyncParser<'static, Z3Parser>) -> Self {
        Self {
            parser,
            parsed: None,
        }
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
        // <div>
            // <h1>{"Axiom Profiler"}</h1>
            <FileDataComponent/>
        // </div>
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

// fn switch(routes: Route) -> Html {
//     match routes {
//         Route::App => html! {
//             <App/>
//         },
//         Route::Test => html! {
//             <Test/>
//         },
//     }
// }

// #[function_component(Main)]
// fn main() -> Html {
//     html! {
//         <BrowserRouter>
//             <Switch<Route> render={switch} />
//         </BrowserRouter>
//     }
// }

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("App is starting");
    // yew::Renderer::<Main>::new().render();
    yew::Renderer::<App>::new().render();
}
