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

#[derive(Clone, PartialEq, Eq)]
pub struct Configuration {
    pub parser: Option<RcParser>,
    pub display: DisplayConfiguration,
}

impl Default for Configuration {
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
            parser: None,
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
        Self {
            config: Configuration::default(),
            update: Callback::from(move |config| link.send_message(config)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        // log::warn!("ConfigurationProvider update: {:?}/{:?}", msg.parser.is_some(), msg.parser.as_ref().map(|p| p.graph.is_some()));
        self.config = msg;
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
