use serde::Deserialize;
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use smt_log_parser::parsers::*;
use std::borrow::Cow;
use std::env;
use std::time::Duration;
use wasm_timer::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = get_settings();
    let filenames = if args.len() < 2 {
        Cow::Owned(vec![settings.file])
    } else {
        Cow::Borrowed(&args[1..])
    };

    for path in &*filenames {
        let path = std::path::Path::new(path);
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();
        if !path.is_file() {
            println!("Skipping {filename:?}");
            continue;
        }
        println!("Parsing {filename:?}");
        let time = Instant::now();
        // // Use to test max loading speed
        // let file = std::fs::read_to_string(path).unwrap();
        // let len = file.chars().filter(|c| *c == '\n').count();
        // let parsed = StreamParser::parse_entire_string(&file, Duration::from_secs_f32(10.0));
        let to = Duration::from_secs_f32(15.0);
        let (_metadata, parser) = path.smt_try_parser_from::<Z3Parser>().unwrap();
        let (timeout, _result) = parser.process_all_timeout(to);
        let elapsed_time = time.elapsed();
        println!(
            "Finished parsing after {} seconds (timeout {timeout})\n",
            elapsed_time.as_secs_f32()
        );
        // result.save_output_to_files(&settings, &time);
        // let render_engine = GraphVizRender;
        // let _svg_result = render_engine.make_svg(OUT_DOT, OUT_SVG);
        // add_link_to_svg(OUT_SVG, OUT_SVG_2);
        // println!(
        //     "Finished render sequence after {} seconds",
        //     time.elapsed().as_secs_f32()
        // );

        // let elapsed_time = time.elapsed();
        // println!("Done, run took {} seconds.", elapsed_time.as_secs_f32());
    }
}

/// Parsing settings.
#[derive(Default, Clone, Debug, Deserialize)]
pub struct Settings {
    // not all settings currently work
    /// Name of file to parse.
    pub file: String,
    /// Whether to consider terms with reused IDs as separate from previous terms with the same ID. Does not work yet.
    pub reuses: bool,
    /// Print contents of parser to standard output item by item.
    pub verbose: bool,
    /// Whether to print all text/json files (Dot and SVG will always be generated regardless of this setting)
    pub save_all_data: bool,
    /// Select a sort type (by line number, cost, depth, etc.)
    /// Does not work yet.
    /// ## TODO
    /// Replace with an enum
    pub sort_by: String,
    /// Stop parsing after a certain amount of time. Does not work yet.
    pub timeout: f32,
    /// Parse only up to a certain number of lines. Does not work yet.
    pub line_limit: usize,
    // add settings for:
    // - number of instantiations to display in final visualization.
}

/// settings file
const SETTINGS: &str = "settings.json";

/// Read settings from `SETTINGS` json file, saving them to a `Settings` struct
pub fn get_settings() -> Settings {
    std::fs::read_to_string(SETTINGS)
        .ok()
        .map(|s| serde_json::from_str(&s).unwrap())
        .unwrap_or_default()
}
