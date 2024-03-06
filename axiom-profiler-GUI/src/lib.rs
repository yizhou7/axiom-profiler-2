use std::cell::RefCell;

use gloo_file::File;
use gloo_file::{callbacks::FileReader, FileList};
use results::svg_result::{RenderingState, SVGResult};
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use smt_log_parser::parsers::{AsyncBufferRead, LogParser, ReaderState};
use wasm_bindgen::JsCast;
use wasm_streams::ReadableStream;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;
use material_yew::{MatButton, MatIcon, MatIconButton, MatDialog, WeakComponentLink};
use material_yew::dialog::{ActionType, MatDialogAction};
use material_yew::linear_progress::MatLinearProgress;

use crate::sidebar::SidebarSectionHeader;

// use crate::svg_result::SVGResult;

// mod filter_chain;
// mod graph;
// mod graph_filters;
// mod input_state;
// mod selected_node;
// mod svg_result;
// mod toggle_switch;
// mod graph_container;
pub mod results;
mod utils;
mod sidebar;

// mod select_dropdown;
pub enum Msg {
    LoadedFile(String, u64, Z3Parser),
    File(Option<File>),
    LoadingState(LoadingState)
}

#[derive(Debug)]
pub enum LoadingState {
    NoFileSelected,
    ReadingToString,
    StartParsing,
    Parsing(ReaderState, u64),
    DoneParsing(bool),
    Rendering(RenderingState),
    FileDisplayed,
}

pub struct OpenedFileInfo {
    file_name: String,
    file_size: u64,
    parser: RcParser,
}

pub struct FileDataComponent {
    file: Option<OpenedFileInfo>,
    readers: Vec<FileReader>,
    pending_ops: usize,
    progress: LoadingState,
}

impl Component for FileDataComponent {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            file: None,
            readers: Vec::new(),
            pending_ops: 0,
            progress: LoadingState::NoFileSelected,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::File(file) => {
                let Some(file) = file else {
                    return false;
                };
                let changed = self.file.is_some() || !self.readers.is_empty();
                self.file = None;
                self.readers.clear();

                let file_name = file.name();
                let file_size = file.size();
                log::info!("Selected file \"{file_name}\"");
                // Turn into stream
                let blob: &web_sys::Blob = file.as_ref();
                let stream = ReadableStream::from_raw(blob.stream().unchecked_into());
                match stream.try_into_async_read() {
                    Ok(stream) => {
                        let link = ctx.link().clone();
                        link.send_message(Msg::LoadingState(LoadingState::StartParsing));
                        let mut parser = Z3Parser::from_async(stream.buffer());
                        wasm_bindgen_futures::spawn_local(async move {
                            log::info!("Parsing \"{file_name}\"");
                            let finished = parser.process_until(|_, state| {
                                if state.lines_read % 100_000 == 0 {
                                    link.send_message(Msg::LoadingState(LoadingState::Parsing(state, file_size)));
                                }
                                state.bytes_read <= 1024 * 1024 * 1024
                            }).await;
                            if finished.is_timeout() {
                                // TODO: make this clear in the UI
                                log::info!("Stopped parsing at 1GB");
                            }
                            link.send_message(Msg::LoadingState(LoadingState::DoneParsing(finished.is_timeout())));
                            link.send_message(Msg::LoadedFile(file_name, file_size, parser.take_parser()))
                        });
                    }
                    Err((_err, _stream)) => {
                        let link = ctx.link().clone();
                        link.send_message(Msg::LoadingState(LoadingState::ReadingToString));
                        let reader = gloo_file::callbacks::read_as_bytes(&file, move |res| {
                            log::info!("Loading to string \"{file_name}\"");
                            let text_data =
                                String::from_utf8(res.expect("failed to read file")).unwrap();
                            log::info!("Parsing \"{file_name}\"");
                            link.send_message(Msg::LoadingState(LoadingState::StartParsing));
                            let mut parser = Z3Parser::from_str(&text_data);
                            let finished = parser.process_until(|_, state| {
                                if state.lines_read % 100_000 == 0 {
                                    link.send_message(Msg::LoadingState(LoadingState::Parsing(state, file_size)));
                                }
                                state.bytes_read <= 512 * 1024 * 1024
                            });
                            if finished.is_timeout() {
                                // TODO: make this clear in the UI
                                log::info!("Stopped parsing at 0.5GB (use Chrome or Firefox to increase this limit to 1GB)");
                            }
                            link.send_message(Msg::LoadingState(LoadingState::DoneParsing(finished.is_timeout())));
                            link.send_message(Msg::LoadedFile(file_name, file_size, parser.take_parser()))
                        });
                        self.readers.push(reader);
                    }
                };
                changed
            }
            Msg::LoadedFile(file_name, file_size, parser) => {
                log::info!("Processing \"{file_name}\"");
                let file = OpenedFileInfo {
                    file_name,
                    file_size,
                    parser: RcParser::new(parser),
                };
                self.file = Some(file);
                true
            }
            Msg::LoadingState(state) => {
                log::info!("New state \"{state:?}\"");
                self.progress = state;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // Parse the timestamp at compile time
        let timestamp =
            chrono::DateTime::parse_from_rfc3339(env!("VERGEN_GIT_COMMIT_TIMESTAMP")).unwrap();
        // Format using https://docs.rs/chrono/latest/chrono/format/strftime/index.html
        let version_info = format!(
            "{} ({})",
            env!("VERGEN_GIT_DESCRIBE"),
            timestamp.format("%H:%M %-d %b %y")
        );
        let version_link = format!(
            "https://github.com/viperproject/axiom-profiler-2/tree/{}",
            env!("VERGEN_GIT_SHA")
        );

        let file_select = NodeRef::default();
        let sidebar = NodeRef::default();
        let scrollable_dialog_link: WeakComponentLink<MatDialog> = WeakComponentLink::default();

        let mut failed = "";
        let loading = match &self.progress {
            LoadingState::NoFileSelected => html!{},
            LoadingState::ReadingToString =>
                html!{<MatLinearProgress indeterminate=true />},
            LoadingState::StartParsing =>
                html!{<MatLinearProgress indeterminate=true />},
            LoadingState::Parsing(state, file_size) => {
                let progress = state.bytes_read as f32 / *file_size as f32;
                html!{<MatLinearProgress progress={progress} buffer={1.0} />}
            }
            LoadingState::DoneParsing(timeout) => {
                if !*timeout {
                    failed = "loading-bar-failed";
                }
                html!{<MatLinearProgress progress={1.0} buffer={1.0} />}
            }
            LoadingState::Rendering(RenderingState::ConstructingGraph) =>
                html!{<MatLinearProgress indeterminate=true />},
            LoadingState::Rendering(RenderingState::GraphToDot) =>
                html!{<MatLinearProgress indeterminate=true />},
            LoadingState::Rendering(RenderingState::RenderingGraph) =>
                html!{<MatLinearProgress indeterminate=true />},
            LoadingState::FileDisplayed => html!{},
        };

        let current_trace = match &self.file {
            Some(file) => {
                let mut idx = 0;
                let mut size = file.file_size;
                let names = ["B", "KB", "MB", "GB", "TB"];
                while size >= 10_000 && idx + 1 < names.len() {
                    size /= 1024;
                    idx += 1;
                }
                let details = format!("{} ({} {})", file.file_name, size, names[idx]);
                html!{
                    <SidebarSectionHeader header_text="Current Trace" collapsed_text="Actions on the current trace"><ul>
                        <li><a id="null" class="trace-file-name">{details}</a></li>
                    </ul></SidebarSectionHeader>
                }
            }
            None => html!{},
        };

        // Callbacks
        let file_select_ref = file_select.clone();
        let on_change = ctx.link().callback(move |_| {
            let files = file_select_ref.cast::<HtmlInputElement>().unwrap().files();
            Msg::File(files.map(FileList::from).and_then(|files| {
                if files.len() == 1 {
                    Some(files[0].clone())
                } else {
                    None
                }
            }))
        });
        let sidebar_ref = sidebar.clone();
        let open_files = self.file.is_some();
        let hide_sidebar = Callback::from(move |_| {
            let sidebar = sidebar_ref.cast::<HtmlElement>().unwrap();
            if sidebar.class_list().contains("hide-sidebar") || open_files {
                let _ = sidebar.class_list().toggle("hide-sidebar");
            }
        });
        let file_select_ref = file_select.clone();
        let select_file = Callback::from(move |click: MouseEvent| {
            click.dyn_into::<Event>().unwrap().prevent_default();
            let input = file_select_ref.cast::<HtmlInputElement>().unwrap();
            input.click();
        });
        let scrollable_dialog_link_clone = scrollable_dialog_link.clone();
        let show_shortcuts = Callback::from(move |click: MouseEvent| {
            click.dyn_into::<Event>().unwrap().prevent_default();
            scrollable_dialog_link_clone.show();
        });
        let progress = ctx.link().callback(|rs| match rs {
            Some(rs) => Msg::LoadingState(LoadingState::Rendering(rs)),
            None => Msg::LoadingState(LoadingState::FileDisplayed),
        });
        html! {
<main>
    <nav class="sidebar" ref={sidebar}>
        <header class="stable"><img src="html/logo_side_small.png" class="brand"/><div class="sidebar-button" onclick={hide_sidebar}><MatIconButton icon="menu"></MatIconButton></div></header>
        <input type="file" ref={file_select} class="trace_file" accept=".log" onchange={on_change} multiple=false/>
        <div class="sidebar-scroll"><div class="sidebar-scroll-container">
            <SidebarSectionHeader header_text="Navigation" collapsed_text="Open or record a new trace"><ul>
                <li><a href="#" onclick={select_file} id="open_trace_file"><div class="material-icons"><MatIcon>{"folder_open"}</MatIcon></div>{"Open trace file"}</a></li>
            </ul></SidebarSectionHeader>
            {current_trace}
            <SidebarSectionHeader header_text="Support" collapsed_text="Documentation & Bugs"><ul>
                <li><a href="#" onclick={show_shortcuts} id="keyboard_shortcuts"><div class="material-icons"><MatIcon>{"help"}</MatIcon></div>{"Keyboard shortcuts"}</a></li>
                <li><a href="https://github.com/viperproject/axiom-profiler-2/blob/main/README.md" target="_blank" id="documentation"><div class="material-icons"><MatIcon>{"find_in_page"}</MatIcon></div>{"Documentation"}</a></li>
                <li><a href="#" id="flags"><div class="material-icons"><MatIcon>{"emoji_flags"}</MatIcon></div>{"Flags"}</a></li>
                <li><a href="https://github.com/viperproject/axiom-profiler-2/issues/new" target="_blank" id="report_a_bug"><div class="material-icons"><MatIcon>{"bug_report"}</MatIcon></div>{"Report a bug"}</a></li>
            </ul></SidebarSectionHeader>
            <div class="sidebar-footer">
                <div title="Number of pending operations" class="dbg-info-square"><div>{"OPS"}</div><div>{self.pending_ops}</div></div>
                <div title="Service Worker: Serving from cache. Ready for offline use" class="dbg-info-square amber"><div>{"SW"}</div><div>{"NA"}</div></div>
                <div class="version"><a href={version_link} title="Channel: stable" target="_blank">{version_info}</a></div>
            </div>
        </div></div>
    </nav>

    <div>
    <div class={failed}>{loading}</div>
        <div style="display: flex; ">
            {self.file.as_ref().map(|f| Self::view_file(RcParser::clone(&f.parser), progress))}
        </div>
    </div>

    // Shortcuts dialog
    <section>
        <MatDialog heading={AttrValue::from("Axiom Profiler Help")} dialog_link={scrollable_dialog_link}>
            {"There are currently no keyboard shortcuts available."}
            <MatDialogAction action_type={ActionType::Primary} action={AttrValue::from("close")}>
                <MatButton label="Close" />
            </MatDialogAction>
        </MatDialog>
    </section>
</main>
        }
    }
}

impl FileDataComponent {
    fn view_file(data: RcParser, progress: Callback<Option<RenderingState>>) -> Html {
        log::debug!("Viewing file");
        html! {
            <SVGResult parser={data} progress={progress}/>
        }
    }
}

#[function_component(App)]
pub fn app() -> Html {
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

pub struct RcParser {
    parser: std::rc::Rc<RefCell<Z3Parser>>,
}

impl std::ops::Deref for RcParser {
    type Target = RefCell<Z3Parser>;

    fn deref(&self) -> &Self::Target {
        &self.parser
    }
}

impl Clone for RcParser {
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone()
        }
    }
}

impl PartialEq for RcParser {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.parser, &*other.parser)
    }
}

impl RcParser {
    fn new(parser: Z3Parser) -> Self {
        Self {
            parser: std::rc::Rc::new(RefCell::new(parser))
        }
    }
}
