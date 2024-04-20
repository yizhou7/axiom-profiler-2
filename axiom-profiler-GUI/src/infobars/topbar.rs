use material_yew::linear_progress::MatLinearProgress;
use smt_log_parser::parsers::z3::graph::RawNodeIndex;
use yew::{function_component, html, Callback, Html, NodeRef, Properties};

use crate::{infobars::{Omnibox, SearchActionResult}, utils::lookup::Kind, LoadingState};

#[derive(PartialEq, Properties)]
pub struct TopbarProps {
    pub progress: LoadingState,
    pub omnibox: NodeRef,
    pub search: Callback<String, Option<SearchActionResult>>,
    pub pick: Callback<(String, Kind), Option<Vec<RawNodeIndex>>>,
    pub select: Callback<RawNodeIndex>,
}

#[function_component]
pub fn Topbar(props: &TopbarProps) -> Html {
    let mut class = "progress progress-anim";
    let mut closed = false;
    let mut indeterminate = false;
    let mut progress = 0.0;
    let mut buffer = 0.0;
    match &props.progress {
        LoadingState::NoFileSelected =>
            closed = true,
        LoadingState::ReadingToString =>
            indeterminate = true,
        LoadingState::StartParsing =>
            indeterminate = true,
        LoadingState::Parsing(parsing, _) => {
            progress = (parsing.reader.bytes_read as f64 / parsing.file_size as f64) as f32;
            buffer = 1.0;
        }
        LoadingState::DoneParsing(timeout, cancelled) => {
            if *timeout && !*cancelled {
                class = "progress progress-anim loading-bar-failed";
            }
            progress = 1.0;
            buffer = 1.0;
        }
        LoadingState::Rendering(..) =>
            indeterminate = true,
        LoadingState::FileDisplayed =>
            closed = true,
    };
    html! {
    <>
        <Omnibox progress={props.progress.clone()} omnibox={props.omnibox.clone()} search={props.search.clone()} pick={props.pick.clone()} select={props.select.clone()} />
        <div {class}><MatLinearProgress {closed} {indeterminate} {progress} {buffer}/></div>
    </>
    }
}
