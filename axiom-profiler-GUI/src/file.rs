use std::{cell::RefCell, rc::Rc};

use gloo::file::File;
use smt_log_parser::{parsers::{AsyncBufferRead, ParseState}, LogParser, Z3Parser};
use wasm_streams::ReadableStream;
use wasm_bindgen::JsCast;
use web_sys::DataTransfer;
use yew::{html::Scope, Callback, DragEvent};

use crate::{global_callbacks::GlobalCallbacks, CallbackRef, FileDataComponent, LoadingState, Msg, ParseProgress, PREVENT_DEFAULT_DRAG_OVER};

impl FileDataComponent {
    pub fn file_drag(registerer: &GlobalCallbacks, link: &Scope<FileDataComponent>) -> [CallbackRef; 3] {
        /// Detects if a file is being dragged over the window
        fn file_drag(event: &DragEvent) -> bool {
            event.data_transfer().as_ref().map(DataTransfer::types)
                .is_some_and(|types|
                    types.iter().flat_map(|ty| ty.as_string()).any(|ty| ty == "Files")
                )
        }
        let last_drag_target = Rc::new(RefCell::new(None));
        let last_drag_target_ref = last_drag_target.clone();
        let drag_enter_ref = (registerer.register_drag_enter)(Callback::from(move |event: DragEvent| {
            *last_drag_target_ref.borrow_mut() = event.target();
            if file_drag(&event) {
                if let Some(mut pd) = PREVENT_DEFAULT_DRAG_OVER.get().and_then(|p| p.lock().ok()) {
                    *pd = true;
                }
                event.prevent_default();
                if let Some(body) = gloo::utils::document().get_element_by_id("body") {
                    body.class_list().add_1("filedrag").ok();
                }
            }
        }));
        fn drag_end() {
            if let Some(body) = gloo::utils::document().get_element_by_id("body") {
                if body.class_list().contains("filedrag") {
                    if let Some(mut pd) = PREVENT_DEFAULT_DRAG_OVER.get().and_then(|p| p.lock().ok()) {
                        *pd = false;
                    }
                    body.class_list().remove_1("filedrag").ok();
                }
            }
        }
        let drag_leave_ref = (registerer.register_drag_leave)(Callback::from(move |event: DragEvent| {
            if file_drag(&event) {
                if *last_drag_target.borrow() == event.target() {
                    drag_end();
                }
            }
        }));
        let link = link.clone();
        let drop_ref = (registerer.register_drop)(Callback::from(move |event: DragEvent| {
            if file_drag(&event) {
                event.prevent_default();
                drag_end();
                let file = event.data_transfer().as_ref()
                    .and_then(DataTransfer::files)
                    .and_then(|files| {
                        (files.length() == 1).then(|| files.get(0).unwrap())
                    }).map(File::from);
                link.send_message(Msg::File(file));
            }
        }));
        [drag_enter_ref, drag_leave_ref, drop_ref]
    }

    pub fn load_opened_file(&mut self, file: File, link: &Scope<FileDataComponent>) -> bool {
        let changed = self.file.is_some();
        drop(self.file.take());
        drop(self.reader.take());

        let file_name = file.name();
        let file_size = file.size();
        log::info!("Selected file \"{file_name}\"");
        *self.cancel.borrow_mut() = false;
        let cancel = self.cancel.clone();
        let cancel_cb = Callback::from(move |_| {
            *cancel.borrow_mut() = true;
        });
        let cancel = self.cancel.clone();
        // Turn into stream
        let blob: &web_sys::Blob = file.as_ref();
        let stream = ReadableStream::from_raw(blob.stream().unchecked_into());
        match stream.try_into_async_read() {
            Ok(stream) => {
                let link = link.clone();
                link.send_message(Msg::LoadingState(LoadingState::StartParsing));
                let mut parser = Z3Parser::from_async(stream.buffer());
                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Parsing \"{file_name}\"");
                    let finished = loop {
                        let mut lines_to_read = 100_000;
                        let finished = parser.process_until(|_, state| {
                            lines_to_read -= 1;
                            let pause = lines_to_read == 0;
                            (pause || *cancel.borrow() || state.bytes_read >= 1024 * 1024 * 1024).then(|| pause)
                        }).await;
                        let ParseState::Paused(true, state) = finished else {
                            break finished;
                        };
                        let parsing = ParseProgress::new(state, file_size);
                        link.send_message(Msg::LoadingState(LoadingState::Parsing(parsing, cancel_cb.clone())));
                        gloo::timers::future::TimeoutFuture::new(0).await;
                    };
                    let cancel = *cancel.borrow();
                    if finished.is_timeout() && !cancel {
                        // TODO: make this clear in the UI
                        log::info!("Stopped parsing at 1GB");
                    }
                    link.send_message(Msg::LoadingState(LoadingState::DoneParsing(finished.is_timeout(), cancel)));
                    link.send_message(Msg::LoadedFile(file_name, file_size, parser.take_parser(), finished, cancel))
                });
            }
            Err((_err, _stream)) => {
                let link = link.clone();
                link.send_message(Msg::LoadingState(LoadingState::ReadingToString));
                let reader = gloo::file::callbacks::read_as_bytes(&file, move |res| {
                    log::info!("Loading to string \"{file_name}\"");
                    let text_data =
                        String::from_utf8(res.expect("failed to read file")).unwrap();
                    log::info!("Parsing \"{file_name}\"");
                    link.send_message(Msg::LoadingState(LoadingState::StartParsing));
                    wasm_bindgen_futures::spawn_local(async move {
                        let mut parser = Z3Parser::from_str(&text_data);
                        let finished = loop {
                            let mut lines_to_read = 100_000;
                            let finished = parser.process_until(|_, state| {
                                lines_to_read -= 1;
                                let pause = lines_to_read == 0;
                                (pause || *cancel.borrow() || state.bytes_read >= 512 * 1024 * 1024).then(|| pause)
                            });
                            let ParseState::Paused(true, state) = finished else {
                                break finished;
                            };
                            let parsing = ParseProgress::new(state, file_size);
                            link.send_message(Msg::LoadingState(LoadingState::Parsing(parsing, cancel_cb.clone())));
                            gloo::timers::future::TimeoutFuture::new(0).await;
                        };
                        let cancel = *cancel.borrow();
                        if finished.is_timeout() && !cancel {
                            // TODO: make this clear in the UI
                            log::info!("Stopped parsing at 0.5GB (use Chrome or Firefox to increase this limit to 1GB)");
                        }
                        link.send_message(Msg::LoadingState(LoadingState::DoneParsing(finished.is_timeout(), cancel)));
                        link.send_message(Msg::LoadedFile(file_name, file_size, parser.take_parser(), finished, cancel))
                    });
                });
                self.reader = Some(reader);
            }
        };
        changed
    }
}
