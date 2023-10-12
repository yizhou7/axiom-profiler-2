use std::sync::{Arc, Mutex};
use crate::parsers::{LogParser, Interruptable};

#[derive(Clone)]

/// A handler for stopping a parsing run early, so that 
/// only the data obtained from lines already read will be saved and/or used for visualizing quantifier instantiations.
/// Needs to run concurrently with the `LogParser` in question to work (e.g. in a separate thread).
pub struct ParseInterrupter {
    /// Mutex obtained from an `Interruptable` parser
    continue_parsing: Arc<Mutex<bool>>
}

impl ParseInterrupter {
    pub fn new(parser: &(impl LogParser + Interruptable)) -> ParseInterrupter {
        ParseInterrupter{ continue_parsing: parser.get_continue_mutex() }
    }

    pub fn stop_parsing(&mut self) {
        match self.continue_parsing.lock() {
            Ok(mut guard) => {
                *guard = false;
            },
            Err(_poisoned) => {}    // don't need to do anything, parser panicked
        }
        println!("Interrupted parsing");
    }
}
