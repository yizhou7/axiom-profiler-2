use gloo::storage::Storage;
use smt_log_parser::display_with::DisplayConfiguration;
use yew::{html, prelude::{Context, Html}, Callback, Children, Component, ContextProvider, Properties};

use crate::RcParser;

// Public

pub trait ConfigurationContext {
    fn get_configuration(&self) -> Option<ConfigurationProvider>;
}
impl<T: Component> ConfigurationContext for html::Scope<T> {
    fn get_configuration(&self) -> Option<ConfigurationProvider> {
        self.context(Callback::noop()).map(|c| c.0)
    }
}

#[derive(Clone, PartialEq)]
pub struct ConfigurationProvider {
    pub config: Configuration,
    pub update: Callback<Configuration>,
}

impl ConfigurationProvider {
    pub fn update_display(mut self, f: impl FnOnce(&mut DisplayConfiguration)) {
        f(&mut self.config.persistent.display);
        self.update.emit(self.config);
    }pub fn update_parser(mut self, f: impl FnOnce(&mut Option<RcParser>)) {
        f(&mut self.config.parser);
        self.update.emit(self.config);
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Configuration {
    pub parser: Option<RcParser>,
    pub persistent: PersistentConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PersistentConfiguration {
    pub display: DisplayConfiguration,
}

impl Default for PersistentConfiguration {
    fn default() -> Self {
        let display = DisplayConfiguration {
            display_term_ids: false,
            display_quantifier_name: false,
            use_mathematical_symbols: true,
            html: true,
            // Set manually elsewhere
            enode_char_limit: 0,
            limit_enode_chars: false,
        };
        Self {
            display,
        }
    }
}

// Private

#[derive(Properties, PartialEq)]
pub struct ConfigurationProviderProps {
    pub children: Children,
}

impl Component for ConfigurationProvider {
    type Message = Configuration;
    type Properties = ConfigurationProviderProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let mut config = Configuration::default();
        if let Ok(cached) = gloo::storage::LocalStorage::get::<PersistentConfiguration>("config") {
            // log::warn!("ConfigurationProvider loaded: {cached:?}");
            config.persistent = cached;
        }
        Self {
            config,
            update: Callback::from(move |config| link.send_message(config)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        // log::warn!("ConfigurationProvider update: {:?}/{:?}: {:?}", msg.parser.is_some(), msg.parser.as_ref().map(|p| p.graph.is_some()), msg.persistent);
        self.config = msg;
        gloo::storage::LocalStorage::set::<&PersistentConfiguration>("config", &self.config.persistent).ok();
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // log::warn!("ConfigurationProvider view: {:?}/{:?}", self.config.parser.is_some(), self.config.parser.as_ref().map(|p| p.graph.is_some()));
        html! {
            <ContextProvider<ConfigurationProvider> context={self.clone()}>
                {ctx.props().children.clone()}
            </ContextProvider<ConfigurationProvider>>
        }
    }
}
