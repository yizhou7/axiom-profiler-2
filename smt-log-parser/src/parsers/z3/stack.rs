use typed_index_collections::TiVec;

use crate::items::StackIdx;

#[derive(Debug, Default, PartialEq)]
pub struct Stack {
    pub(super) stack: Vec<StackIdx>,
    pub(super) stack_frames: TiVec<StackIdx, StackFrame>,
}

impl Stack {
    pub(super) fn new_frame(&mut self, idx: usize) {
        assert_eq!(idx, self.stack.len());
        let idx = self.stack_frames.push_and_get_key(StackFrame::new());
        self.stack.push(idx);
    }

    pub(super) fn pop_frame(&mut self, idx: usize) -> &StackFrame {
        assert_eq!(idx, self.stack.len());
        let idx = self.stack.pop().unwrap();
        self.stack_frames[idx].active = false;
        &self.stack_frames[idx]
    }

    pub(super) fn active_frame(&self) -> Option<StackIdx> {
        self.stack.last().copied()
    }
}

#[derive(Debug, PartialEq)]
pub struct StackFrame {
    pub active: bool,
    // pub new_terms: Vec<(Option<TermIdx>, TermIdx)>,
}

impl StackFrame {
    pub fn new() -> Self {
        Self { active: true }
    }
}
