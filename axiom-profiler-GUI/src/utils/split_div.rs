use yew::{html, prelude::{Context, Html}, Children, Component, MouseEvent, NodeRef, Properties};

use crate::{CallbackRef, GlobalCallbacksContext, PagePosition};

#[derive(Properties, PartialEq)]
pub struct SplitDivProps {
    /// 0.0 to 1.0
    pub initial_position: f64,
    pub left_bound: f64,
    pub right_bound: f64,
    /// Must be spaced apart by at least 0.05
    pub snap_positions: Vec<f64>,
    pub children: Children,
}

pub struct SplitDiv {
    position: f64,

    dragging: bool,
    container: NodeRef,

    _callback_refs: [CallbackRef; 2],
}

#[allow(dead_code)]
pub enum Msg {
    MouseDown(MouseEvent),
    MouseMove(MouseEvent),
    MouseUp(MouseEvent),
}

impl Component for SplitDiv {
    type Message = Msg;

    type Properties = SplitDivProps;

    fn create(ctx: &Context<Self>) -> Self {
        let registerer = ctx.link().get_callbacks_registerer().unwrap();
        let mouse_move_ref = (registerer.register_mouse_move)(ctx.link().callback(Msg::MouseMove));
        let mouse_up_ref = (registerer.register_mouse_up)(ctx.link().callback(Msg::MouseUp));
        let _callback_refs = [mouse_move_ref, mouse_up_ref];

        Self {
            position: ctx.props().initial_position,
            dragging: false,
            container: NodeRef::default(),
            _callback_refs,
        }
    }
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::MouseDown(ev) => {
                ev.prevent_default();
                self.dragging = true;
                false
            }
            Msg::MouseMove(ev) => {
                if !self.dragging || ev.buttons() != 1 {
                    self.dragging = false;
                    return false;
                }
                ev.prevent_default();
                let position = PagePosition { x: ev.client_x(), y: ev.client_y() };
                let container = self.container.cast::<web_sys::Element>().unwrap();
                let container = container.get_bounding_client_rect();
                self.position = (position.x as f64 - container.left()) / container.width();
                true
            }
            Msg::MouseUp(_) => {
                self.dragging = false;
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let middle_width = 6.0;
        let onmousedown = ctx.link().callback(Msg::MouseDown);

        let mut position = self.position;
        for snap in &ctx.props().snap_positions {
            if (snap - self.position).abs() < 0.025 {
                position = *snap;
            }
        }

        let position = position.min(ctx.props().right_bound).max(ctx.props().left_bound);
        let style_left = format!("width:calc({}% - {}px); height:100%;", position * 100.0, middle_width * position);

        let style_middle = format!("width:{}px; height:100%; cursor:ew-resize; background-color:rgb(229, 231, 235); background-repeat:no-repeat; background-position:50% center; background-image:url(\"/html/vertical.png\");", middle_width);

        let inverse = 1.0 - position;
        let style_right = format!("width:calc({}% - {}px); height:100%;", inverse * 100.0, middle_width * position);
        let mut children = ctx.props().children.iter();
        html! {
            <div ref={&self.container} style="display:flex; flex-direction:row; width:100%; height:100%">
                <div style={style_left}>
                    {children.next()}
                </div>
                <div style={style_middle} {onmousedown}></div>
                <div style={style_right}>
                    {for children}
                </div>
            </div>
        }
    }
}
