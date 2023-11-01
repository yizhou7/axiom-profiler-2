use gloo_file::{callbacks::FileReader, File, FileList};
use smt_log_parser::parsers::LogParser;
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;
use std::{collections::HashMap, ops::Deref};
use std::str::from_utf8;
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;
use crate::svg_result::*;

mod svg_result;
mod graph;
mod graph_state;
mod input_state;
pub enum Msg {
    LoadedBytes(String, Vec<u8>),
    Files(Option<FileList>),
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
                let Some(files) = files else {
                    return true;
                };
                log::info!("Files selected: {}", files.len());
                for file in files.into_iter() {
                    let file_name = file.name();
                    // Turn into stream
                    let blob: &web_sys::Blob = file.as_ref();
                    let stream = ReadableStream::from_raw(blob.stream().unchecked_into());
                    let read = stream.into_async_read();
                    let mut read = Z3Parser::from_async(read);
                    wasm_bindgen_futures::spawn_local(async move {
                        // TODO: read file here
                        // let result = read.process_all().await;
                    });
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
            let files = e.target_dyn_into::<HtmlInputElement>().unwrap().files();
            Msg::Files(files.map(FileList::from))
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

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
        <div>
            <h1>{"Axiom Profiler"}</h1>
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
