use web_sys::HtmlElement;
use yew::{function_component, html, Callback, Children, Html, NodeRef, Properties};

#[derive(PartialEq, Properties)]
pub struct SidebarSectionHeaderProps {
    pub header_text: String,
    pub collapsed_text: String,
    pub children: Children,
}

#[function_component]
pub fn SidebarSectionHeader(props: &SidebarSectionHeaderProps) -> Html {
    let section = NodeRef::default();
    let section_ref = section.clone();
    let collapse = Callback::from(move |_| {
        let section: HtmlElement = section_ref.cast::<HtmlElement>().unwrap();
        let _ = section.class_list().toggle("expanded");
    });
    html! {
        <section class="expanded" ref={section}>
            <div class="section-header" onclick={collapse}><h1 title={props.collapsed_text.clone()}>{&props.header_text}</h1><h2>{&props.collapsed_text}</h2></div>
            <div class="section-content">{props.children.clone()}</div>
        </section>
    }
}
