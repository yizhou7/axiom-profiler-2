use scraper::{self, Selector, Html};

enum Tag {
    Svg,
    SvgG,
    Polygon,
    TopLevelPolygon,
    Ellipse,
    Text,
    Path,
}

pub trait TagAttributes {
    fn get_attributes(html_input: &Html) -> Self;
}

pub struct SvgAttr {
    pub xmlns: String, 
    pub width: String, 
    pub height: String, 
    pub view_box: String, 
}

pub struct TopLevelGAttr {
    pub id: String, 
    pub class: String, 
    pub transform: String, 
}

pub struct TopLevelPolygonAttr {
    pub fill: String,
    pub stroke: String,
    pub points: String,
}

pub struct PolygonAttr {
    pub fill: String,
    pub stroke: String,
    pub points: String,
}

pub struct EllipseAttr {
    pub fill: String,
    pub stroke: String,
    pub cx: String,
    pub cy: String,
    pub rx: String,
    pub ry: String,
}

pub struct TextAttr {
    pub text_anchor: String,
    pub x: String,
    pub y: String,
    pub font_family: String,
    pub font_size: String,
}

pub struct PathAttr {
    pub fill: String,
    pub stroke: String,
    pub d: String,
}

impl TagAttributes for SvgAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::Svg, html_input);
        Self {
            xmlns: "http://www.w3.org/2000/svg".to_string(),
            width: get_attr("width"), 
            height: get_attr("height"), 
            view_box: get_attr("viewBox"), 
        }
    }
}

impl TagAttributes for TopLevelGAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::SvgG, html_input);
        Self {
            id: get_attr("id"), 
            class: get_attr("class"), 
            transform: get_attr("transform"),
        }
    }
}

impl TagAttributes for PolygonAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::Polygon, html_input);
        Self {
            fill: get_attr("fill"), 
            stroke: get_attr("stroke"), 
            points: get_attr("points"),
        }
    }
} 

impl TagAttributes for TopLevelPolygonAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::TopLevelPolygon, html_input);
        Self {
            fill: get_attr("fill"), 
            stroke: get_attr("stroke"), 
            points: get_attr("points"),
        }
    }
}

impl TagAttributes for EllipseAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::Ellipse, html_input);
        Self {
            fill: get_attr("fill"),
            stroke: get_attr("stroke"),
            cx: get_attr("cx"),
            cy: get_attr("cy"),
            rx: get_attr("rx"),
            ry: get_attr("ry"),
        }
    }
}

impl TagAttributes for TextAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::Text, html_input);
        Self {
            text_anchor: get_attr("text-anchor"),
            x: get_attr("x"),
            y: get_attr("y"),
            font_family: get_attr("font-family"),
            font_size: get_attr("font-size"),
        }
    }
}

impl TagAttributes for PathAttr {
    fn get_attributes(html_input: &Html) -> Self {
        let get_attr = attribute_getter_for_tag(Tag::Path, html_input);
        Self {
            fill: get_attr("fill"),
            stroke: get_attr("stroke"),
            d: get_attr("d"),
        }
    }
}

fn attribute_getter_for_tag<'a>(tag: Tag, html_input: &'a Html) -> impl Fn(&str) -> String + 'a {
    let selector = match tag {
        Tag::Svg => Selector::parse("svg").unwrap(),
        Tag::SvgG => Selector::parse("svg > g").unwrap(),
        Tag::TopLevelPolygon => Selector::parse("svg > g > polygon").unwrap(),
        Tag::Polygon => Selector::parse("polygon").unwrap(),
        Tag::Ellipse => Selector::parse("ellipse").unwrap(),
        Tag::Text => Selector::parse("text").unwrap(),
        Tag::Path => Selector::parse("path").unwrap(),
    }; 
    let tag = scraper::Html::select(html_input, &selector)
        .next()
        .expect("Tag not found in HTML")
        .value();

    move |attr: &str| tag.attr(attr).unwrap().to_string()
}