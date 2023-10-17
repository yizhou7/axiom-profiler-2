use yew::prelude::*;
use crate::attribute_fetcher::*;
use crate::nodes::*;
use crate::edges::*;
use crate::svg_result::GraphState;

#[function_component(Graph)]
pub fn graph() -> Html {
    let state = use_context::<GraphState>().expect("no ctx found");
    // extract svg attributes
    let svg = scraper::Html::parse_document(&state.svg_text);
    let SvgAttr {xmlns, width, height, view_box} = SvgAttr::get_attributes(&svg);
    // extract top-level g attributes
    let TopLevelGAttr {id, class, transform} = TopLevelGAttr::get_attributes(&svg);
    // extract top-level polygon attributes
    let TopLevelPolygonAttr {fill, stroke, points} = TopLevelPolygonAttr::get_attributes(&svg); 
    html! {
        <svg {xmlns} {width} {height} viewBox={view_box}>
            <g {id} {class} {transform}>
                <polygon {fill} {stroke} {points}></polygon>
                <Nodes />
                <Edges />
            </g>
        </svg>
    }
}