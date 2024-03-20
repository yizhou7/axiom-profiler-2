use material_yew::linear_progress::MatLinearProgress;
use yew::{function_component, html, AttrValue, Callback, Html, MouseEvent, Properties};

use crate::{results::svg_result::RenderingState, LoadingState, SIZE_NAMES};

#[derive(PartialEq, Properties)]
pub struct TopbarProps {
    pub progress: LoadingState,
}

#[function_component]
pub fn Topbar(props: &TopbarProps) -> Html {
    let mut failed = "";
    let mut omnibox_info = None;
    let mut icon = None;
    let mut callback = None;
    log::info!("Got progress: {:?}", props.progress);
    let loading = match &props.progress {
        LoadingState::NoFileSelected =>
            html!{<MatLinearProgress closed=true />},
        LoadingState::ReadingToString => {
            omnibox_info = Some(AttrValue::from("Loading trace"));
            html!{<MatLinearProgress indeterminate=true />}
        }
        LoadingState::StartParsing => {
            omnibox_info = Some(AttrValue::from("Parsing trace"));
            html!{<MatLinearProgress indeterminate=true />}
        }
        LoadingState::Parsing(parsing, cancel) => {
            icon = Some("stop_circle");
            callback = Some(cancel);
            let progress = parsing.reader.bytes_read as f64 / parsing.file_size as f64;
            let info = if let Some(mut speed) = &parsing.speed {
                let mut idx = 0;
                while speed >= 10_000.0 && idx + 1 < SIZE_NAMES.len() {
                    speed /= 1024.0;
                    idx += 1;
                }
                format!("Parsing trace {:.0}% - {:.0} {}/s", progress * 100.0, speed, SIZE_NAMES[idx])
            } else {
                format!("Parsing trace {:.0}%", progress * 100.0)
            };
            omnibox_info = Some(AttrValue::from(info));
            html!{<MatLinearProgress progress={progress as f32} buffer={1.0} />}
        }
        LoadingState::DoneParsing(timeout, cancelled) => {
            if *timeout && !*cancelled {
                failed = "loading-bar-failed";
            }
            html!{<MatLinearProgress progress={1.0} buffer={1.0} />}
        }
        LoadingState::Rendering(RenderingState::ConstructingGraph, timeout, _) |
        LoadingState::Rendering(RenderingState::ConstructedGraph, timeout, _) => {
            if *timeout {
                omnibox_info = Some(AttrValue::from("Analysing partial trace"));
            } else {
                omnibox_info = Some(AttrValue::from("Analysing trace"));
            }
            html!{<MatLinearProgress indeterminate=true />}
        }
        LoadingState::Rendering(RenderingState::GraphToDot, _, _) => {
            omnibox_info = Some(AttrValue::from("Rendering trace"));
            html!{<MatLinearProgress indeterminate=true />}
        }
        LoadingState::Rendering(RenderingState::RenderingGraph, _, _) => {
            omnibox_info = Some(AttrValue::from("Rendering trace"));
            html!{<MatLinearProgress indeterminate=true />}
        }
        LoadingState::FileDisplayed =>
            html!{<MatLinearProgress closed=true />},
    };
    let omnibox_disabled = omnibox_info.is_some();
    let icon = icon.unwrap_or(if omnibox_disabled { "info" } else { "search" });
    let icon = if let Some(callback) = callback {
        let callback = callback.clone();
        let onclick = Callback::from(move |click: MouseEvent| {
            click.prevent_default();
            callback.emit(());
        });
        html! { <a href="#" class="icon" onclick={onclick}>{icon}</a> }
    } else {
        html! { <div class="icon">{icon}</div> }
    };
    let placeholder = omnibox_info.unwrap_or(AttrValue::from("Search or type '>' for commands"));

    html! {
    <>
        <div class="omnibox">
            {icon}
            <input placeholder={placeholder} readonly={omnibox_disabled} disabled={omnibox_disabled}/>
        </div>
        <div class={format!("progress progress-anim {failed}")}>{loading}</div>
    </>
    }
}
