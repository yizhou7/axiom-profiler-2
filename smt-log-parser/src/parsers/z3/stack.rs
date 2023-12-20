use typed_index_collections::TiVec;

use crate::items::StackIdx;

#[derive(Debug, Default)]
pub struct Stack {
    pub(super) stack: Vec<StackIdx>,
    pub(super) stack_frames: TiVec<StackIdx, StackFrame>,
}

impl Stack {
    fn add_frame(&mut self) {
        let idx = self.stack_frames.push_and_get_key(StackFrame::new());
        self.stack.push(idx);
    }
    fn remove_frame(&mut self, active: bool) -> Option<StackIdx> {
        let idx = self.stack.pop()?;
        self.stack_frames[idx].active = active;
        Some(idx)
    }
    #[must_use]
    fn ensure_height(&mut self, height: usize) -> Option<()> {
        let mut res = Some(());
        // Neither condition should hold, but handle it as best we can.
        while height > self.stack.len() {
            // Have not run into this case, so make tests fail if it happens.
            res = None;
            self.add_frame();
        }
        while height < self.stack.len() {
            // This can happen when pushing a new frame in e.g. z3 v4.8.17 and
            // v4.11.2.
            // It seems that there is a bug where the pop doesn't get emitted
            // and so we need to conservatively leak the frame and treat it as
            // always active.
            self.remove_frame(true);
        }
        res
    }

    #[must_use]
    pub(super) fn new_frame(&mut self, idx: usize) -> Option<()> {
        let res = self.ensure_height(idx);
        self.add_frame();
        res
    }

    #[must_use]
    pub(super) fn pop_frames(&mut self, count: usize, idx: usize) -> Option<()> {
        debug_assert!(0 < count && count <= idx);
        let res = self.ensure_height(idx);
        for _ in 0..count {
            self.remove_frame(false)?;
        }
        res
    }

    pub(super) fn active_frame(&self) -> Option<StackIdx> {
        self.stack.last().copied()
    }
}

#[derive(Debug)]
pub struct StackFrame {
    pub active: bool,
}

impl StackFrame {
    pub fn new() -> Self {
        Self { active: true }
    }
}
