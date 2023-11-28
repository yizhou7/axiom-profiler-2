fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("App is starting");
    // yew::Renderer::<Main>::new().render();
    yew::Renderer::<axiom_profiler_gui::App>::new().render();
}
