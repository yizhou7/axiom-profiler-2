use yew::{prelude::*, virtual_dom::VNode};
use web_sys::HtmlInputElement;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlElement, SvgsvgElement};
use wasm_bindgen::prelude::Closure;
use gloo_console::log;

pub enum Msg {
    SetValueTo(usize),
    Noop,
}

pub struct GraphStruct {
    svg_result: Option<VNode>,
    div_ref: NodeRef,
    closures: Vec<Closure<dyn Fn(Event)>>,
    zoom_factor: usize,
}

#[derive(Properties, PartialEq, Default)]
pub struct GraphStructProps {
    pub svg_text: AttrValue,
    pub update_selected_node: Callback<usize>,
}

impl Component for GraphStruct {
    type Message = Msg;
    type Properties = GraphStructProps;

    fn create(_ctx: &Context<Self>) -> Self {
        log!("creating GraphStruct component");
        Self {
            svg_result: None,
            div_ref: NodeRef::default(),
            closures: Vec::new(),
            zoom_factor: 100,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetValueTo(value) => {
                self.zoom_factor = value;
                let div = self.div_ref
                    .cast::<HtmlElement>()
                    .expect("div_ref not attached to div element");
                // setting the transform-origin to the top left corner of the surrounding div
                // we know there is only a single SVG element, hence can just take item(0)
                let svg_element = div.get_elements_by_tag_name("svg").item(0);
                if let Some(el) = svg_element {
                    let svg_el = el.dyn_into::<SvgsvgElement>().ok().unwrap();
                    let _ = svg_el.set_attribute("transform-origin", "top left");
                    web_sys::console::log_1(&"Updating the transform-attribute of svg in update()".into());
                    let _ = svg_el.set_attribute("style", format!("transform: scale({});", self.zoom_factor as f32 / 100.0).as_str());
                }
                true
            },
            Msg::Noop => false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, _first_render: bool) {
        if let Some(div) = self.div_ref.cast::<HtmlElement>() {
            // setting the transform-origin to the top left corner of the surrounding div
            // we know there is only a single SVG element, hence can just take item(0)
            let svg_element = div.get_elements_by_tag_name("svg").item(0);
            if let Some(el) = svg_element {
                let svg_el = el.dyn_into::<SvgsvgElement>().ok().unwrap();
                let _ = svg_el.set_attribute("transform-origin", "top left");
                web_sys::console::log_1(&"Updating the transform-attribute of svg with zoom-factor in rendered():".into());
                web_sys::console::log_1(&self.zoom_factor.into());
                let _ = svg_el.set_attribute("style", format!("transform: scale({});", self.zoom_factor as f32 / 100.0).as_str());
            }
            // construct event_listeners that emit node indices (contained in title tags)
            let descendant_nodes = div.get_elements_by_class_name("node");
            let closures: Vec<Closure<dyn Fn(Event)>> = (0..descendant_nodes.length())
                .map(|i| {
                    // extract node_index from node to construct callback that emits it
                    let node = descendant_nodes.item(i).unwrap();
                    let title_element = node
                        .query_selector("title")
                        .expect("Failed to select title element")
                        .unwrap();
                    let title_content = title_element.text_content().unwrap();
                    let node_index = title_content.parse::<usize>().unwrap();
                    let callback = ctx.props().update_selected_node.clone();
                    let closure: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
                        callback.emit(node_index);
                    });
                    // attach event listener to node
                    node.add_event_listener_with_callback(
                        "click",
                        closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                    closure
                })
                .collect();
            self.closures = closures;
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        web_sys::console::log_1(&"Props have changed!".into());
        self.svg_result = Some(Html::from_html_unchecked(ctx.props().svg_text.clone()));
        // let div = self.div_ref
        //     .cast::<HtmlElement>()
        //     .expect("div_ref not attached to div element");
        // // setting the transform-origin to the top left corner of the surrounding div
        // // we know there is only a single SVG element, hence can just take item(0)
        // let svg_element = div.get_elements_by_tag_name("svg").item(0);
        // if let Some(el) = svg_element {
        //     let svg_el = el.dyn_into::<SvgsvgElement>().ok().unwrap();
        //     let _ = svg_el.set_attribute("transform-origin", "top left");
        //     web_sys::console::log_1(&"Updating the transform-attribute of svg".into());
        //     let _ = svg_el.set_attribute("style", format!("transform: scale({});", self.zoom_factor as f32 / 100.0).as_str());
        // }

        // // construct event_listeners that emit node indices (contained in title tags)
        // let descendant_nodes = div.get_elements_by_class_name("node");
        // for i in 0..descendant_nodes.length() {
        //     // extract node_index from node to construct callback that emits it
        //     let node = descendant_nodes.item(i).unwrap();
        //     let title_element = node
        //         .query_selector("title")
        //         .expect("Failed to select title element")
        //         .unwrap();
        //     let title_content = title_element.text_content().unwrap();
        //     let node_index = title_content.parse::<usize>().unwrap();
        //     let callback = ctx.props().update_selected_node.clone();
        //     let closure: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
        //         callback.emit(node_index);
        //     });
        //     // attach event listener to node
        //     node.add_event_listener_with_callback(
        //         "click",
        //         closure.as_ref().unchecked_ref(),
        //     ).unwrap()
        // }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let set_value = {
            move |input_event: Event| {
                let target: HtmlInputElement = input_event
                    .target()
                    .unwrap_throw()
                    .dyn_into()
                    .unwrap_throw();
                match target.value().to_string().parse::<usize>() {
                    Ok(value) => {
                        log::debug!("Setting the value to {}", value);
                        Msg::SetValueTo(value)
                    }
                    Err(_) => {
                        Msg::SetValueTo(100)
                    }
                }
            }
        };
        let set_value_on_enter = ctx.link().callback(
            move |key_event: KeyboardEvent| {
                if key_event.key() == "Enter" {
                    let event: Event = key_event.clone().into();
                    set_value(event)
                } else {
                    Msg::Noop
                }
            }
        );
        let set_value_on_blur = ctx.link().callback(
            move |blur_event: FocusEvent| {
                let event: Event = blur_event.clone().into();
                set_value(event)
            }
        );
        
        html! {
        <div style="flex: 70%; height: 85vh; overflow: auto; position: relative;">
            <div style="position: sticky; top: 0px; left: 0px; z-index: 1">
                <label for="input">{"Zoom factor in %: "}</label>
                <input onkeypress={set_value_on_enter} onblur={set_value_on_blur} id="input" placeholder="100"/>
            </div>
            <div ref={self.div_ref.clone()}>
                {if let Some(ref vnode) = self.svg_result {
                    html! {vnode.clone()}
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
    }
}