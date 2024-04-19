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
    let mut failed = "";
    log::info!("Got progress: {:?}", props.progress);
    let loading = match &props.progress {
        LoadingState::NoFileSelected =>
            html!{<MatLinearProgress closed=true />},
        LoadingState::ReadingToString =>
            html!{<MatLinearProgress indeterminate=true />},
        LoadingState::StartParsing =>
            html!{<MatLinearProgress indeterminate=true />},
        LoadingState::Parsing(parsing, _) => {
            let progress = parsing.reader.bytes_read as f64 / parsing.file_size as f64;
            html!{<MatLinearProgress progress={progress as f32} buffer={1.0} />}
        }
        LoadingState::DoneParsing(timeout, cancelled) => {
            if *timeout && !*cancelled {
                failed = "loading-bar-failed";
            }
            html!{<MatLinearProgress progress={1.0} buffer={1.0} />}
        }
        LoadingState::Rendering(..) =>
            html!{<MatLinearProgress indeterminate=true />},
        LoadingState::FileDisplayed =>
            html!{<MatLinearProgress closed=true />},
    };
    html! {
    <>
        <Omnibox progress={props.progress.clone()} omnibox={props.omnibox.clone()} search={props.search.clone()} pick={props.pick.clone()} select={props.select.clone()} />
        <div class={format!("progress progress-anim {failed}")}>{loading}</div>
    </>
    }
}
