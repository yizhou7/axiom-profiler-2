use std::time::Duration;

use gloo_file::{callbacks::FileReader, FileList};
use smt_log_parser::parsers::{LogParser, AsyncBufferRead, AsyncCursorRead, AsyncParser};
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;
use web_sys::{Event, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;
use crate::graph::{Graph, GraphProps};

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
                            let reader = gloo_file::callbacks::read_as_bytes(file, move |res| {
                                link.send_message(Msg::LoadedBytes(
                                    file_name,
                                    res.expect("failed to read file"),
                                ))
                            });
                            self.readers.push(reader);
                        }
                    };
                }
                changed
            }
            Msg::LoadedBytes(file_name, data) => {
                log::info!("Processing: {}", file_name);
                let parser = Z3Parser::from_async(data.into_async_cursor());
                self.parsers.push(ParserData::new(parser));
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
                {for self.parsers.iter().map(|parser_data| Self::parse(parser_data))}
                </div>
            </div>
        }
    }
}

impl FileDataComponent {
    fn parse(parser_data: &ParserData) -> Html {
        log::debug!("Parsing log");
        let parse_log = Callback::from(|_| {
                wasm_bindgen_futures::spawn_local(
                    async move {
                        let result = parser_data.parser.process_check_every(Duration::new(1,0), |_, _| false).await;
                    },
                );
            });
        html! {
            // contains parser options (max line nr.) that user can specify
            // contains button "Parse log" that allows user to start parsing log
            // calls process_check_every to stop at the max line nr.
            // contains progress-bar that shows how many lines have already been processed
            // the process_check_every updates the progress-bar after each delta
        <>
            // <div>
            //     <UsizeInput label={"Parse log up to which line number? "} input_value={max_log_line_nr} />
            //     <UsizeInput label={"Parse log up to how many instantiations? "} input_value={max_instantiations} />
                <button onclick={parse_log}>{"Parse log"}</button>
            // </div>
            // <Graph svg_text={svg_text.clone()} line_nr_of_node={line_nr_of_node.clone()} /> 
        </>


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
