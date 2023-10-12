/// Private module for generating colors
mod colors {
    /// Generate `n` distinct colors in HSL format; each color represents instantiations from the same quantifier.
    pub fn make_hsl_strings(n: usize) -> Vec<String> {
        const DEFAULT_SAT: f64 = 100.0;
        const DEFAULT_LUM: f64 = 70.0;
        generate_colors(n).iter().map(|hue| 
            format!("hsl({}, {}%, {}%)", hue, DEFAULT_SAT, DEFAULT_LUM)).collect::<Vec<String>>()
    }

    /// Generate `n` distinct colors (hue values) from golden ratio; essentially pseudorandomly with fixed seed.
    /// 
    /// Explained here: https://martin.ankerl.com/2009/12/09/how-to-create-random-colors-programmatically/
    /// ### Note
    /// Colors became hard to tell apart once `n` gets large enough (> 10?)
    fn generate_colors(n: usize) -> Vec<f64> {
        const GOLDEN_RATIO_RECIPROCAL: f64 = 0.618033988749895;
        let mut hue = 0.0;
        let mut result = vec![hue];
        for _ in 1..n {
            hue += GOLDEN_RATIO_RECIPROCAL;
            hue = hue.fract() * 360.0;
            result.push(hue);
        }
        result
    }
}

pub mod make_css {
    use std::collections::HashSet;
    use super::colors::*;
    const NODE_HOVER_RULE: &str = ".node:hover {\n\topacity: 0.6\n}\n\n";
    const EDGE_HOVER_RULE: &str = ".edge:hover * {\n\topacity: 0.4\n}\n\n";
    /// From given set of quantifier names, generate a CSS style sheet as a string: 
    /// fixed rules for SVG nodes and edges
    /// followed by a fill for each ellipse class (representing one quantifier and color).
    pub fn make_css_string(quants: &HashSet<String>) -> String {
        let mut result = String::from(NODE_HOVER_RULE) + EDGE_HOVER_RULE;
        let rgb_strings = make_hsl_strings(quants.len());
        for (i, quant) in quants.iter().enumerate() {
            result += &format!(".{} ellipse {{\n\tfill: {} !important\n}}\n\n", quant, rgb_strings[i]);
        }
        result
    }
}