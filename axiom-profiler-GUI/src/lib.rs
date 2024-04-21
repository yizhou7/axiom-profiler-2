use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock, RwLock};

use fxhash::{FxHashMap, FxHashSet};
use gloo_file::File;
use gloo_file::{callbacks::FileReader, FileList};
use petgraph::visit::EdgeRef;
use results::svg_result::{Msg as SVGMsg, RenderedGraph, RenderingState, SVGResult};
use smt_log_parser::parsers::z3::graph::{InstGraph, VisibleEdgeIndex, RawNodeIndex};
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use smt_log_parser::parsers::{ParseState, ReaderState};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_timer::Instant;
use web_sys::{HtmlElement, HtmlInputElement};
use yew::prelude::*;
use yew_router::prelude::*;
use material_yew::{MatIcon, MatIconButton, MatDialog, WeakComponentLink};

use crate::configuration::{ConfigurationContext, ConfigurationProvider};
use crate::filters::FiltersState;
use crate::infobars::{SidebarSectionHeader, Topbar};

pub use global_callbacks::{GlobalCallbacksProvider, CallbackRef, GlobalCallbacksContext};

pub mod results;
mod utils;
mod infobars;
mod filters;
mod global_callbacks;
pub mod configuration;
pub mod homepage;
pub mod shortcuts;
pub mod file;

pub const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
pub fn version() -> Option<semver::Version> {
    let version = GIT_DESCRIBE.strip_prefix("v")?;
    semver::Version::parse(version).ok().filter(|v| v.pre.is_empty())
}

const SIZE_NAMES: [&'static str; 5] = ["B", "KB", "MB", "GB", "TB"];
const ALLOW_HIDE_SIDEBAR_NO_FILE: bool = true;

pub static MOUSE_POSITION: OnceLock<RwLock<PagePosition>> = OnceLock::new();
pub fn mouse_position() -> &'static RwLock<PagePosition> {
    MOUSE_POSITION.get_or_init(|| RwLock::new(PagePosition { x: 0, y: 0 }))
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position<T> {
    pub x: T,
    pub y: T,
}
pub type PagePosition = Position<i32>;
pub type PrecisePosition = Position<f64>;

pub static PREVENT_DEFAULT_DRAG_OVER: OnceLock<Mutex<bool>> = OnceLock::new();

pub enum Msg {
    File(Option<File>),
    LoadedFile(String, u64, Z3Parser, ParseState, bool),
    LoadingState(LoadingState),
    RenderedGraph(RenderedGraph),
    SelectedNodes(Vec<RawNodeIndex>),
    SelectedEdges(Vec<VisibleEdgeIndex>),
    KeyDown(KeyboardEvent),
    ShowHelpToggled(bool),
    SearchMatchingLoops,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadingState {
    NoFileSelected,
    ReadingToString,
    StartParsing,
    Parsing(ParseProgress, Callback<()>),
    // Stopped early, cancelled?
    DoneParsing(bool, bool),
    Rendering(RenderingState, bool, bool),
    FileDisplayed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseProgress {
    reader: ReaderState,
    file_size: u64,
    time: Instant,
    bytes_delta: Option<usize>,
    time_delta: Option<std::time::Duration>,
    speed: Option<f64>,
}
impl ParseProgress {
    pub fn new(reader: ReaderState, file_size: u64) -> Self {
        Self {
            reader,
            file_size,
            time: Instant::now(),
            bytes_delta: None,
            time_delta: None,
            speed: None,
        }
    }
    pub fn delta(&mut self, old: &Self) {
        assert!(self.reader.bytes_read > old.reader.bytes_read);
        if self.reader.bytes_read < old.reader.bytes_read {
            *self = old.clone();
            return;
        }
        // Value >= 0.0, the higher the value the more smoothed out the speed is
        // (but also takes longer to react to changes in speed)
        const SPEED_SMOOTHING: f64 = 10.0;
        let bytes_delta = self.reader.bytes_read - old.reader.bytes_read;
        self.bytes_delta = Some(bytes_delta);
        let time_delta = self.time - old.time;
        self.time_delta = Some(time_delta);
        let speed = bytes_delta as f64 / time_delta.as_secs_f64();
        self.speed = Some(old.speed.map(|old| (speed + SPEED_SMOOTHING * old) / (SPEED_SMOOTHING + 1.0)).unwrap_or(speed));
    }
}

#[derive(Clone)]
pub struct OpenedFileInfo {
    file_name: String,
    file_size: u64,
    parser_state: ParseState,
    parser_cancelled: bool,
    update: Rc<RefCell<Result<Callback<SVGMsg>, Vec<SVGMsg>>>>,
    filter: WeakComponentLink<FiltersState>,
    selected_nodes: Vec<RawNodeIndex>,
    selected_edges: Vec<VisibleEdgeIndex>,
    rendered: Option<RenderedGraph>,
}

impl PartialEq for OpenedFileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
            && self.file_size == other.file_size
            && std::mem::discriminant(&self.parser_state) == std::mem::discriminant(&other.parser_state)
            && self.selected_nodes == other.selected_nodes
            && self.selected_edges == other.selected_edges
            && self.rendered.as_ref().map(|r| r.graph.generation) == other.rendered.as_ref().map(|r| r.graph.generation)
    }
}

impl OpenedFileInfo {
    pub fn send_update(&self, msg: SVGMsg) {
        match &mut *self.update.borrow_mut() {
            Ok(cb) => cb.emit(msg),
            Err(e) => e.push(msg),
        }
    }
    pub fn send_updates(&self, msgs: impl Iterator<Item = SVGMsg>) {
        match &mut *self.update.borrow_mut() {
            Ok(cb) => for msg in msgs {
                cb.emit(msg);
            },
            Err(e) => e.extend(msgs),
        }
    }
}

pub struct FileDataComponent {
    file_select: NodeRef,
    file: Option<OpenedFileInfo>,
    reader: Option<FileReader>,
    pending_ops: usize,
    progress: LoadingState,
    cancel: Rc<RefCell<bool>>,
    navigation_section: NodeRef,
    help_dialog: WeakComponentLink<MatDialog>,
    showing_help: bool,
    omnibox: NodeRef,
    sidebar_button: NodeRef,
    _callback_refs: [CallbackRef; 6],
}

impl Component for FileDataComponent {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let help_dialog = WeakComponentLink::default();
        let registerer = ctx.link().get_callbacks_registerer().unwrap();
        let mouse_move_ref = (registerer.register_mouse_move)(Callback::from(|event: MouseEvent| {
            *mouse_position().write().unwrap() = PagePosition { x: event.client_x(), y: event.client_y() };
        }));
        let pd = PREVENT_DEFAULT_DRAG_OVER.get_or_init(|| Mutex::default());
        let drag_over_ref = (registerer.register_drag_over)(Callback::from(|event: DragEvent| {
            *mouse_position().write().unwrap() = PagePosition { x: event.client_x(), y: event.client_y() };
            let pd = *pd.lock().unwrap();
            if pd {
                event.prevent_default();
            }
        }));
        let [drag_enter_ref, drag_leave_ref, drop_ref] = Self::file_drag(&registerer, ctx.link());
        let keydown = (registerer.register_keyboard_down)(ctx.link().callback(Msg::KeyDown));
        let _callback_refs = [mouse_move_ref, drag_over_ref, drag_enter_ref, drag_leave_ref, drop_ref, keydown];
        Self {
            file_select: NodeRef::default(),
            file: None,
            reader: None,
            pending_ops: 0,
            progress: LoadingState::NoFileSelected,
            cancel: Rc::default(),
            navigation_section: NodeRef::default(),
            help_dialog,
            showing_help: false,
            omnibox: NodeRef::default(),
            sidebar_button: NodeRef::default(),
            _callback_refs,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::File(file) => {
                let Some(file) = file else {
                    return false;
                };
                // reset the configuration to default
                let cfg = ctx.link().get_configuration().unwrap();
                cfg.update_parser(|p| *p = None);

                self.load_opened_file(file, ctx.link())
            }
            Msg::LoadingState(mut state) => {
                log::info!("New state \"{state:?}\"");
                if let (LoadingState::Parsing(parsing, _), LoadingState::Parsing(old, _)) = (&mut state, &self.progress) {
                    parsing.delta(old);
                }
                self.progress = state;
                true
            }
            Msg::RenderedGraph(rendered) => {
                if let Some(file) = &mut self.file {
                    let old_len = file.selected_nodes.len();
                    file.selected_nodes.retain(|n| rendered.graph.contains(*n));
                    if file.selected_nodes.len() != old_len {
                        ctx.link().send_message(Msg::SelectedNodes(file.selected_nodes.clone()));
                    }
                    if let Some(old) = &file.rendered {
                        let old_len = file.selected_edges.len();
                        // Update selected edges
                        let mut to_update: FxHashMap<_, _> = file.selected_edges.iter_mut().flat_map(|e| {
                            let old_edge = &old.graph[*e];
                            let different = !rendered.graph.graph.edge_weight(e.0).is_some_and(|edge| edge == old_edge);
                            different.then(|| (old_edge, e))
                        }).collect();
                        if !to_update.is_empty() {
                            for new_edge in rendered.graph.graph.edge_references() {
                                if let Some(e) = to_update.remove(new_edge.weight()) {
                                    *e = VisibleEdgeIndex(new_edge.id());
                                    if to_update.is_empty() {
                                        break;
                                    }
                                }
                            }
                            if !to_update.is_empty() {
                                let to_remove: FxHashSet<_> = to_update.into_iter().map(|(_, v)| *v).collect();
                                file.selected_edges.retain(|e| !to_remove.contains(e));
                            }
                        }
                        if file.selected_edges.len() != old_len {
                            ctx.link().send_message(Msg::SelectedEdges(file.selected_edges.clone()));
                        }
                    }
                    file.rendered = Some(rendered);
                }
                true
            }
            Msg::LoadedFile(file_name, file_size, parser, parser_state, parser_cancelled) => {
                log::info!("Processing \"{file_name}\"");
                drop(self.reader.take());
                let parser = RcParser::new(parser);
                let cfg = ctx.link().get_configuration().unwrap();
                cfg.update_parser(|p| *p = Some(RcParser::clone(&parser)));
                let file = OpenedFileInfo {
                    file_name,
                    file_size,
                    parser_state,
                    parser_cancelled,
                    filter: WeakComponentLink::default(),
                    update: Rc::new(RefCell::new(Err(Vec::new()))),
                    selected_nodes: Vec::new(),
                    selected_edges: Vec::new(),
                    rendered: None,
                };
                self.file = Some(file);
                if let Some(navigation_section) = self.navigation_section.cast::<web_sys::Element>() {
                    let _ = navigation_section.class_list().remove_1("expanded");
                }
                true
            }
            Msg::SelectedNodes(nodes) => {
                let Some(file) = &mut self.file else {
                    return false
                };
                file.selected_nodes = nodes;
                true
            }
            Msg::SelectedEdges(edges) => {
                let Some(file) = &mut self.file else {
                    return false
                };
                file.selected_edges = edges;
                true
            }
            Msg::ShowHelpToggled(showing_help) => {
                self.showing_help = showing_help;
                false
            }
            Msg::KeyDown(event) => match event.key().as_str() {
                "?" => {
                    if self.showing_help {
                        self.help_dialog.close();
                    } else {
                        self.help_dialog.show();
                    }
                    false
                }
                "s" => {
                    if event.meta_key() {
                        event.prevent_default();
                        self.omnibox.cast::<HtmlElement>().and_then(|omnibox| omnibox.focus().ok());
                    }
                    false
                }
                "b" => {
                    if event.meta_key() {
                        event.prevent_default();
                        if let Some(sidebar_button) = self.sidebar_button.cast::<HtmlElement>() {
                            sidebar_button.click()
                        }
                    }
                    false
                }
                _ => false,
            }
            Msg::SearchMatchingLoops => {
                if let Some(file) = &mut self.file {
                    // TODO: re-add finding matching loops
                    // if let Some(g) = file.parser.graph.borrow_mut().as_mut() {
                    //     file.parser.found_mls = Some(g.search_matching_loops());
                    //     return true;
                    // }
                }
                false
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
        let is_canary = version().is_none();

        let sidebar = NodeRef::default();

        let current_trace = match &self.file {
            Some(file) => {
                let search_matching_loops = ctx.link().callback(|_| Msg::SearchMatchingLoops);
                html!{
                    <FiltersState file={file.clone()} search_matching_loops={search_matching_loops}/>
                }
            }
            None => html!{},
        };

        // Callbacks
        let file_select_ref = self.file_select.clone();
        let on_change = ctx.link().callback(move |_| {
            let files = file_select_ref.cast::<HtmlInputElement>().unwrap().files();
            Msg::File(files.map(FileList::from).and_then(|files|
                (files.len() == 1).then(|| files[0].clone())
            ))
        });
        let sidebar_ref = sidebar.clone();
        let open_files = self.file.is_some();
        let hide_sidebar = Callback::from(move |_| {
            if let Some(sidebar) = sidebar_ref.cast::<HtmlElement>() {
                if ALLOW_HIDE_SIDEBAR_NO_FILE || sidebar.class_list().contains("hide-sidebar") || open_files {
                    sidebar.class_list().toggle("hide-sidebar").ok();
                }
            }
        });
        let help_dialog_clone = self.help_dialog.clone();
        let show_shortcuts = Callback::from(move |click: MouseEvent| {
            click.prevent_default();
            help_dialog_clone.show();
        });
        let onopened = ctx.link().callback(|_| Msg::ShowHelpToggled(true));
        let onclosed = ctx.link().callback(|_| Msg::ShowHelpToggled(false));
        let page = self.file.as_ref().map(|f| {
            let (timeout, cancel) = (f.parser_state.is_timeout(), f.parser_cancelled);
            let link = ctx.link().clone();
            let progress = ctx.link().callback(move |rs| match rs {
                Err(rs) => Msg::LoadingState(LoadingState::Rendering(rs, timeout, cancel)),
                Ok(rendered) => {
                    link.send_message(Msg::RenderedGraph(rendered));
                    Msg::LoadingState(LoadingState::FileDisplayed)
                }
            });
            let selected_nodes = ctx.link().callback(Msg::SelectedNodes);
            let selected_edges = ctx.link().callback(Msg::SelectedEdges);
            Self::view_file(f.clone(), progress, selected_nodes, selected_edges)
        }).unwrap_or_else(|| {
            html!{<homepage::Homepage {is_canary}/>}
        });
        let header_class = if is_canary { "canary" } else { "stable" };
        html! {
<>
    <nav class="sidebar" ref={sidebar}>
        <header class={header_class}><img src="html/logo_side_small.png" class="brand"/><div ref={&self.sidebar_button} class="sidebar-button" onclick={hide_sidebar}><MatIconButton icon="menu"></MatIconButton></div></header>
        <input type="file" ref={&self.file_select} class="trace_file" accept=".log" onchange={on_change} multiple=false/>
        <div class="sidebar-scroll"><div class="sidebar-scroll-container">
            <SidebarSectionHeader header_text="Navigation" collapsed_text="Open a new trace" section={self.navigation_section.clone()}><ul>
                <li><a href="#" draggable="false" id="open_trace_file"><div class="material-icons"><MatIcon>{"folder_open"}</MatIcon></div>{"Open trace file"}</a></li>
            </ul></SidebarSectionHeader>
            {current_trace}
            <SidebarSectionHeader header_text="Support" collapsed_text="Documentation & Bugs"><ul>
                <li><a href="#" draggable="false" onclick={show_shortcuts} id="keyboard_shortcuts"><div class="material-icons"><MatIcon>{"help"}</MatIcon></div>{"Keyboard shortcuts"}</a></li>
                <li><a href="https://github.com/viperproject/axiom-profiler-2/blob/main/README.md" target="_blank" id="documentation"><div class="material-icons"><MatIcon>{"find_in_page"}</MatIcon></div>{"Documentation"}</a></li>
                <li><a href="#" draggable="false" id="flags"><div class="material-icons"><MatIcon>{"emoji_flags"}</MatIcon></div>{"Flags"}</a></li>
                <li><a href="https://github.com/viperproject/axiom-profiler-2/issues/new" target="_blank" id="report_a_bug"><div class="material-icons"><MatIcon>{"bug_report"}</MatIcon></div>{"Report a bug"}</a></li>
            </ul></SidebarSectionHeader>
            <div class="sidebar-footer">
                <div title="Number of pending operations" class="dbg-info-square"><div>{"OPS"}</div><div>{self.pending_ops}</div></div>
                <div title="Service Worker: Serving from cache. Ready for offline use" class="dbg-info-square amber"><div>{"SW"}</div><div>{"NA"}</div></div>
                <div class="version"><a href={version_link} title="Channel: stable" target="_blank">{version_info}</a></div>
            </div>
        </div></div>
    </nav>
    <div class="topbar">
        <Topbar progress={self.progress.clone()} omnibox={self.omnibox.clone()} />
    </div>
    <div class="alerts"></div>
    {page}

    // Shortcuts dialog
    <shortcuts::Shortcuts noderef={self.help_dialog.clone()} {onopened} {onclosed}/>
</>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            // Do this instead of `onclick` when creating `open_trace_file`
            // above. Otherwise we run into the error here:
            // https://github.com/leptos-rs/leptos/issues/2104 due to the `.click()`.
            let input = self.file_select.cast::<HtmlInputElement>().unwrap();
            let closure: Closure<dyn Fn(MouseEvent)> = Closure::new(move |e: MouseEvent| {
                e.prevent_default(); input.click();
            });
            let div = gloo::utils::document().get_element_by_id("open_trace_file").unwrap();
            div.add_event_listener_with_callback(
                "click",
                closure.as_ref().unchecked_ref(),
            ).unwrap();
            closure.forget();
        }
    }
}

impl FileDataComponent {
    fn view_file(data: OpenedFileInfo, progress: Callback<Result<RenderedGraph, RenderingState>>, selected_nodes: Callback<Vec<RawNodeIndex>>, selected_edges: Callback<Vec<VisibleEdgeIndex>>) -> Html {
        html! {
            <div class="page">
                <SVGResult file={data} {progress} {selected_nodes} {selected_edges}/>
            </div>
        }
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <main><GlobalCallbacksProvider>
            <ConfigurationProvider><FileDataComponent/></ConfigurationProvider>
        </GlobalCallbacksProvider></main>
    }
}

pub struct RcParser {
    parser: Rc<Z3Parser>,
    graph: Option<Rc<RefCell<InstGraph>>>,
    found_mls: Option<usize>,
}

impl Clone for RcParser {
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            graph: self.graph.clone(),
            found_mls: self.found_mls,
        }
    }
}

impl PartialEq for RcParser {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.parser, &*other.parser)
        && self.graph.is_some() == other.graph.is_some()
        && self.found_mls == other.found_mls
    }
}
impl Eq for RcParser {}

impl RcParser {
    fn new(parser: Z3Parser) -> Self {
        Self {
            parser: Rc::new(parser),
            graph: None,
            found_mls: None,
        }
    }
}
