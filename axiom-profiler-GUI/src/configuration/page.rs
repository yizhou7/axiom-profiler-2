use std::rc::Rc;

use wasm_bindgen::JsCast;
use yew::{function_component, use_context, Callback, Html, Event, use_effect_with_deps};

use crate::configuration::{ConfigurationProvider, PersistentConfiguration};

macro_rules! flag_widget {
    ($cfg:ident, $($access:ident).+, $($from:literal => $to:literal),+; $title:expr, $description:expr) => {
        {
            let id = stringify!(cfg.$($access).+);
            let curr = &(($cfg).config.persistent.$($access).+);
            let curr_to = match curr {
                $($from => $to,)+
            };
            let effect = move |curr_to| {
                let element = gloo::utils::document().get_element_by_id(id);
                let element = element.and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok());
                if let Some(element) = element {
                    element.set_value(curr_to);
                };
                || {}
            };
            let deps = curr_to;
            let default = PersistentConfiguration::default_const().$($access).+;
            let default_to = match &default {
                $($from => $to,)+
            };
            let cfg_update = $cfg.update.clone();
            let onchange = Callback::from(move |e: Event| {
                e.prevent_default();
                let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) else {
                    return;
                };
                cfg_update.update(move |cfg| {
                    let new_value = match target.value().as_str() {
                        $($to => $from,)+
                        _ => unreachable!(),
                    };
                    let old_value = &mut cfg.persistent.$($access).*;
                    if old_value != &new_value {
                        *old_value = new_value;
                        true
                    } else {
                        false
                    }
                });
            });
            (yew::html! {
                <div class="flag-widget">
                    <label>{$title}</label>
                    <select {id} {onchange}>
                        $(<option value={$to}>{if $to == default_to { concat!("Default (", $to, ")") } else { $to }}</option>)+
                    </select>
                    <div class="description">{$description}</div>
                </div>
            }, effect, deps)
        }
    };
}

#[function_component]
pub fn Flags(_props: &()) -> Html {
    let cfg = use_context::<Rc<ConfigurationProvider>>().unwrap();
    let cfg_update = cfg.update.clone();
    let reset = Callback::from(move |_| cfg_update.update(|cfg| {
        let new = PersistentConfiguration::default();
        if cfg.persistent != new {
            cfg.persistent = new;
            true
        } else {
            false
        }
    }));
    let (display_term_ids, effect, deps) = flag_widget!(
        cfg,
        display.display_term_ids,
        true => "Enabled",
        false => "Disabled";
        "Display term IDs",
        "Display the IDs (e.g. `#123`) of the terms as they appear in the log file in the UI."
    );
    use_effect_with_deps(move |deps| effect(deps), deps);

    yew::html! {
        <div class="flags-page"><div class="flags-content">
            <h1>{"Configuration flags"}</h1>
            <button onclick={reset}>{"Reset configuration"}</button>
            {display_term_ids}
        </div></div>
    }
}
