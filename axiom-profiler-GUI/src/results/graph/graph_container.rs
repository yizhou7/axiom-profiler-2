use petgraph::graph::{EdgeIndex, NodeIndex};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Element, HtmlInputElement, ResizeObserver, ResizeObserverEntry};
use yew::prelude::*;

use crate::results::svg_result::RenderedGraph;
use crate::{CallbackRef, GlobalCallbacksContext, PagePosition, PrecisePosition};

use super::svg_graph::{Graph, Svg};

pub enum Msg {
    SetValueTo(f32),
    SetScrollTo((PrecisePosition, PrecisePosition)),
    Wheel(WheelEvent),
    Scroll(Event),
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
    Resize(Vec<ResizeObserverEntry>),
}

pub struct GraphContainer {
    graph: Option<(Html, u32)>,
    window: GraphWindow,

    mouse_closures: Option<Closure<dyn Fn(MouseEvent)>>,
    resize_observer: Option<(ResizeObserver, Closure<dyn Fn(Vec<ResizeObserverEntry>)>)>,
    drag_start: Option<(PagePosition, PagePosition, bool)>,
    zoom_factor: f32,
    zoom_factor_delta: f32,
    _callback_refs: [CallbackRef; 2],
}

struct GraphWindow {
    scroll_window: NodeRef,
    /// The scroll position
    graph_position: PrecisePosition,
    /// The scrollable area
    graph_dims: PrecisePosition,
    /// The window position
    window_position: PrecisePosition,
    /// The window size
    window_dims: PrecisePosition,
}

impl GraphWindow {
    pub fn new() -> GraphWindow {
        GraphWindow {
            scroll_window: NodeRef::default(),
            graph_position: PrecisePosition { x: 0.0, y: 0.0 },
            graph_dims: PrecisePosition { x: 0.0, y: 0.0 },
            window_position: PrecisePosition { x: 0.0, y: 0.0 },
            window_dims: PrecisePosition { x: 0.0, y: 0.0 },
        }
    }
    fn scroll_by(&mut self, x: f64, y: f64) {
        let pos = PrecisePosition { x: self.graph_position.x + x, y: self.graph_position.y + y };
        self.scroll_to(pos);
    }
    fn scroll_to(&mut self, pos: PrecisePosition) {
        let new_x = pos.x.min(self.graph_dims.x - self.window_dims.x).max(0.0);
        let new_y = pos.y.min(self.graph_dims.y - self.window_dims.y).max(0.0);
        self.update_scroll_position(new_x, new_y);
        self.scroll_window.cast::<Element>().unwrap_throw().scroll_to_with_x_and_y(new_x, new_y);
    }

    fn update_scroll_position(&mut self, new_x: f64, new_y: f64) {
        self.graph_position.x = new_x;
        self.graph_position.y = new_y;
    }

    fn read_scroll_position(&mut self) {
        let scroll_window = self.scroll_window.cast::<Element>().unwrap_throw();
        let new_x = scroll_window.scroll_left() as f64;
        let new_y = scroll_window.scroll_top() as f64;
        self.update_scroll_position(new_x, new_y);
    }
    fn read_window_dimensions(&mut self) {
        let scroll_window = self.scroll_window.cast::<Element>().unwrap_throw();
        let rect = scroll_window.get_bounding_client_rect();
        let new_position = PrecisePosition { x: rect.x(), y: rect.y() };
        self.window_position = new_position;
        let new_dims = PrecisePosition { x: rect.width(), y: rect.height() };
        self.window_dims = new_dims;
    }
}

#[derive(Properties, PartialEq)]
pub struct GraphContainerProps {
    pub rendered: Option<RenderedGraph>,
    pub update_selected_nodes: Callback<NodeIndex>,
    pub update_selected_edges: Callback<EdgeIndex>,
    pub deselect_all: Callback<()>,
    pub selected_nodes: Vec<NodeIndex>,
    pub selected_edges: Vec<EdgeIndex>,
}

impl Component for GraphContainer {
    type Message = Msg;
    type Properties = GraphContainerProps;

    fn create(ctx: &Context<Self>) -> Self {
        let graph = ctx.props().rendered.as_ref().map(|r| (Html::from_html_unchecked(r.svg_text.clone()), r.graph.generation));
        let registerer = ctx.link().get_callbacks_registerer().unwrap();
        let mouse_move_ref = (registerer.register_mouse_move)(ctx.link().callback(Msg::MouseMove));
        let mouse_up_ref = (registerer.register_mouse_up)(ctx.link().callback(Msg::MouseUp));
        let _callback_refs = [mouse_move_ref, mouse_up_ref];
        Self { graph, mouse_closures: None, resize_observer: None, drag_start: None, window: GraphWindow::new(), zoom_factor: 1.0, zoom_factor_delta: 1.0, _callback_refs }
    }
    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        self.graph = ctx.props().rendered.as_ref().map(|r| (Html::from_html_unchecked(r.svg_text.clone()), r.graph.generation));
        self.zoom_factor_delta = 1.0;
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetValueTo(value) => {
                let zoom_factor = value.min(5.0).max(0.005);
                self.zoom_factor_delta = zoom_factor / self.zoom_factor;
                self.zoom_factor = zoom_factor;
                true
            }
            Msg::SetScrollTo((pos, graph_dims)) => {
                self.window.graph_dims = graph_dims;
                self.window.scroll_to(pos);
                false
            }
            Msg::Wheel(ev) => {
                if ev.ctrl_key() {
                    ev.prevent_default();
                    let need_to_flip = ev.delta_y() >= 0.0;
                    // We need `delta < 0` such that the `powf` doesn't overflow
                    let delta = -ev.delta_y().abs() as f32 * SPREAD;
                    const SPREAD: f32 = 0.5;
                    let e_pow = std::f32::consts::E.powf(delta);
                    let sigmoid = (1.0 - e_pow) / (1.0 + e_pow);
                    const MAX_DEV: f32 = 0.1;
                    let factor = if need_to_flip {
                        1.0 - sigmoid * MAX_DEV
                    } else {
                        1.0 + sigmoid * MAX_DEV
                    };
                    let zoom_factor = (self.zoom_factor * factor).min(5.0).max(0.005);
                    self.zoom_factor_delta = zoom_factor / self.zoom_factor;
                    self.zoom_factor = zoom_factor;
                    true
                } else {
                    ev.prevent_default();
                    self.window.scroll_by(ev.delta_x(), ev.delta_y());
                    false
                }
            }
            Msg::Scroll(_) => {
                self.window.read_scroll_position();
                false
            }
            Msg::MouseDown(ev) => {
                let pos = PagePosition { x: ev.client_x(), y: ev.client_y() };
                self.drag_start = Some((pos, pos, false));
                false
            }
            Msg::MouseMove(ev) => {
                if ev.buttons() != 1 {
                    self.drag_start = None;
                } else {
                    if let Some((start, last, drag)) = &mut self.drag_start {
                        let pos = PagePosition { x: ev.client_x(), y: ev.client_y() };
                        if (start.x - pos.x).abs() > 5 || (start.y - pos.y).abs() > 5 {
                            *drag = true;
                            let last_last = *last;
                            *last = pos;
                            if !ev.shift_key() {
                                let (dx, dy) = ((last_last.x - last.x) as f64, (last_last.y - last.y) as f64);
                                self.window.scroll_by(dx, dy);
                            }
                        }
                    }
                }
                false
            }
            Msg::MouseUp(_) => {
                if let Some((_, _, drag)) = self.drag_start.take() {
                    if !drag {
                        ctx.props().deselect_all.emit(());
                    }
                }
                false
            }
            Msg::Resize(_resizes) => {
                self.window.read_window_dimensions();
                self.window.read_scroll_position();
                // If we are not doing the initial resize where all the dims are setup
                // if !resizes.is_empty() { do_scrolling_here_if_necessary }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let input = NodeRef::default();
        let set_value = {
            move |input_event: Event| {
                let target: HtmlInputElement = input_event
                    .target()
                    .unwrap_throw()
                    .dyn_into()
                    .unwrap_throw();
                match target.value().to_string().parse::<f32>() {
                    Ok(value) => {
                        Msg::SetValueTo(value)
                    }
                    Err(_) => Msg::SetValueTo(1.0),
                }
            }
        };
        let link = ctx.link().clone();
        let input_ref = input.clone();
        let set_value_on_enter = Callback::from(move |key_event: KeyboardEvent| {
            if key_event.key() == "Enter" {
                let target = input_ref.cast::<HtmlInputElement>().unwrap_throw();
                let msg = match target.value().parse::<f32>() {
                    Ok(value) => Msg::SetValueTo(value),
                    Err(_) => Msg::SetValueTo(1.0),
                };
                link.send_message(msg);
            }
        });
        let set_value_on_blur = ctx.link().callback(move |blur_event: FocusEvent| {
            let event: Event = blur_event.clone().into();
            set_value(event)
        });
        let onwheel = ctx.link().callback(Msg::Wheel);
        let onscroll = ctx.link().callback(Msg::Scroll);
        let zoom_factor = format!("{:.3}", self.zoom_factor);
        let idx = zoom_factor.chars().rev().position(|c| c != '0' && c != '.').unwrap_or(zoom_factor.len() - 1);
        let zoom_factor = zoom_factor[0..zoom_factor.len() - idx].to_string();
        let set_scroll = ctx.link().callback(Msg::SetScrollTo);
        html! {
        <div ref={&self.window.scroll_window} style="height: 100%; overflow: auto; overscroll-behavior-x: none;" {onwheel} {onscroll}>
            <div style="position: absolute; bottom: 0; left: 0; z-index: 1;">
                <label for="input">{"Zoom factor: "}</label>
                <input ref={input} onkeypress={set_value_on_enter} onblur={set_value_on_blur} id="input" size="5" value={zoom_factor}/>
            </div>
            <Graph
                rendered={ctx.props().rendered.clone()}
                update_selected_nodes={&ctx.props().update_selected_nodes}
                update_selected_edges={&ctx.props().update_selected_edges}
                zoom_factor={self.zoom_factor}
                zoom_factor_delta={self.zoom_factor_delta}
                selected_nodes={ctx.props().selected_nodes.clone()}
                selected_edges={ctx.props().selected_edges.clone()}
                scroll_position={self.window.graph_position.clone()}
                set_scroll={set_scroll}
                scroll_window={self.window.scroll_window.clone()}
            ><Svg svg={self.graph.clone()}/></Graph>
        </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let mouse_down = ctx.link().callback(Msg::MouseDown);
            let mouse_down: Closure<dyn Fn(MouseEvent)> = Closure::new(move |ev| mouse_down.emit(ev));
            // attach event listener to node
            let div = self.window.scroll_window.cast::<Element>().unwrap_throw();
            div.add_event_listener_with_callback(
                "mousedown",
                mouse_down.as_ref().unchecked_ref(),
            ).unwrap_throw();
            self.mouse_closures = Some(mouse_down);

            // Resize observer
            let resize_closure = ctx.link().callback(Msg::Resize);
            resize_closure.emit(Vec::new());
            let resize_closure = Closure::new(move |entries: Vec<ResizeObserverEntry>| {
                resize_closure.emit(entries)
            });
            let observer = ResizeObserver::new(resize_closure.as_ref().unchecked_ref()).unwrap_throw();
            observer.observe(&div);
            self.resize_observer = Some((observer, resize_closure));
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        if let Some(mouse_down) = self.mouse_closures.take() {
            let div = self.window.scroll_window.cast::<Element>().unwrap_throw();
            div.remove_event_listener_with_callback(
                "mousedown",
                mouse_down.as_ref().unchecked_ref(),
            ).unwrap();
        }
        if let Some((observer, _closure)) = self.resize_observer.take() {
            observer.disconnect();
        }
    }
}
