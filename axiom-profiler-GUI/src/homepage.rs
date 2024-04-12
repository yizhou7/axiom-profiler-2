use yew::{function_component, html, Callback, Html, Properties};

#[derive(Properties, Clone, PartialEq)]
pub struct HomepageProps {
    pub is_canary: bool,
}

#[function_component]
pub fn Homepage(props: &HomepageProps) -> Html {
    let is_canary = props.is_canary;
    let stable = Callback::from(move |_| {
        if is_canary {
            gloo::utils::window().location().set_href("/axiom-profiler-2/").unwrap();
        }
    });
    let canary = Callback::from(move |_| {
        if !is_canary {
            gloo::utils::window().location().set_href("/axiom-profiler-2/canary/").unwrap();
        }
    });
    html! {
        <div class="page home-page"><div class="home-page-center">
            <div class="home-page-title">
                <img src="html/logo_small.png" class="logo" />{"Axiom Profiler"}</div>
            <div class="home-page-hints">
                // <div class="tagline">{"New!"}</div>
                // <ul>
                //     <li>{"New updated "}<a href="LINK_TO_DOCS" class="pf-anchor">{"tabs"}</a>{" are extensible and user friendly."}</li>
                // </ul>
            </div>
            <div class="channel-select">
                <div>{"Feeling adventurous? Try our bleeding edge Canary version"}</div>
                <fieldset>
                    <input type="radio" name="chan" id="chan_stable" checked={!is_canary} />
                    <label for="chan_stable" onclick={stable}>{"stable"}</label>
                    <input type="radio" name="chan" id="chan_canary" checked={is_canary} />
                    <label for="chan_canary" onclick={canary}>{"canary"}</label>
                    <div class="highlight"></div>
                </fieldset>
            </div>
        // Can add a footer here
        </div><a href="#" target="_blank" class="footer">{""}</a></div>
    }
}
