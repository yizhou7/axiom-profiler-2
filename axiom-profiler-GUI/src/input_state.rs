use web_sys::HtmlInputElement;
use yew::prelude::*;
use wasm_bindgen::{UnwrapThrowExt, JsCast};

pub enum InputAction {
    SetValueTo(i32),
    ReadInput(i32),
    ResetState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputValue {
    pub value: i32,
    input: i32,
}

impl Default for InputValue {
    fn default() -> Self {
        Self {
            value: i32::MAX,
            input: i32::MAX,
        }
    }
}

impl Reducible for InputValue {
    type Action = InputAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            InputAction::SetValueTo(value) => Self {
                value,
                input: self.input,
            }.into(),
            InputAction::ReadInput(input) => Self {
                value: self.value,
                input,
            }.into(),
            InputAction::ResetState => {
                Self::default().into()
            },
        }
    }
}

#[derive(Properties, PartialEq)] 
pub struct IntegerInputProps where {
    pub label: AttrValue,
    pub dependency: AttrValue,
    pub input_value: UseReducerHandle<InputValue>,
}

#[function_component(IntegerInput)]
pub fn integer_input(props: &IntegerInputProps) -> Html {
    let input_value = props.input_value.clone(); 
    let input_ref = use_node_ref();

    let read_input = Callback::from({
        let input_value = input_value.clone();
        move |input_event: InputEvent| {
            let target: HtmlInputElement = input_event
                .target()
                .unwrap_throw()
                .dyn_into()
                .unwrap_throw();
            if let Ok(input) = target.value().to_string().parse::<i32>() {
                input_value.dispatch(InputAction::ReadInput(input));
            } else {
                // by default, i.e., if user input can't be parsed as i32, we
                // reset the state of the graph
                input_value.dispatch(InputAction::ResetState);
            }
        }
    });

    let set_value = Callback::from({
        let input_value = input_value.clone();
        move |key_event: KeyboardEvent| {
            let event_type = key_event.type_();
            log::debug!("Event type: {event_type}");
            if key_event.key() == "Enter" {
                let value = input_value.input;
                input_value.dispatch(InputAction::SetValueTo(value));
            }
       }
    });

    let set_value_on_blur = Callback::from({
        let input_value = input_value.clone();
        move |event: FocusEvent| {
            let event_type = event.type_();
            log::debug!("Event type: {event_type}");
            let value = input_value.input;
            input_value.dispatch(InputAction::SetValueTo(value));
       }
    });

    {
        // Whenever dependency changes, need to reset the state
        let dep = props.dependency.clone();
        let input_value = input_value.clone();
        let input_ref = input_ref.clone();
        use_effect_with(dep, {
            let input_value = input_value.clone();
            move |_| {
                input_value.dispatch(InputAction::ResetState);
                let input = input_ref
                    .cast::<HtmlInputElement>()
                    .expect("div_ref not attached to div element");
                input.set_value("");
            }
        });
    }

    html! {
        <div>
            <label for="input">{props.label.to_string()}</label>
            <input ref={input_ref} type="number" oninput={read_input} onkeypress={set_value} onblur={set_value_on_blur} id="input" />
        </div>
    }
}