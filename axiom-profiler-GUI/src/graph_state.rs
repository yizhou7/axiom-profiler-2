use yew::prelude::*;

pub enum GUIAction {
    SetMaxLineNr(i32),
    ReadInput(i32),
    ResetState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphState {
    pub max_line_nr: i32,
    pub input: i32,
}

impl Default for GraphState {
    fn default() -> Self {
        Self { 
            max_line_nr: i32::MAX,
            input: i32::MAX,
        }
    }
}

impl Reducible for GraphState {
    type Action = GUIAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            GUIAction::SetMaxLineNr(max_line_nr) => Self {
                max_line_nr,
                input: self.input,
            }.into(),
            GUIAction::ReadInput(input) => Self {
                max_line_nr: self.max_line_nr,
                input,
            }.into(),
            GUIAction::ResetState => Self::default().into(),
        }
    }
}