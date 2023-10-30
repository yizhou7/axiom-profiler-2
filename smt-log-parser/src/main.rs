use smt_log_parser::file_io::*;
use smt_log_parser::parsers::*;
use smt_log_parser::parsers::z3::z3parser::Z3Parser;
use smt_log_parser::render::*;
use std::env;
use std::time::Duration;
use wasm_timer::Instant;

// default output files
const OUT_DOT: &str = "out/output.dot";
const OUT_SVG_2: &str = "out/output2.svg";
const OUT_SVG: &str = "out/output.svg";

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = get_settings();
    let filename = if args.len() < 2 {
        &settings.file
    } else {
        &args[1]
    };

    let time = Instant::now();
    let parsed = StreamParser::parse_entire_file(filename, Duration::from_secs_f32(10.0));
    let (_timeout, mut result): (_, Z3Parser) = parsed.unwrap();
    let elapsed_time = time.elapsed();
    println!(
        "Finished parsing after {} seconds",
        elapsed_time.as_secs_f32()
    );
    result.save_output_to_files(&settings, &time);
    let render_engine = GraphVizRender;
    let _svg_result = render_engine.make_svg(OUT_DOT, OUT_SVG);
    add_link_to_svg(OUT_SVG, OUT_SVG_2);
    println!(
        "Finished render sequence after {} seconds",
        time.elapsed().as_secs_f32()
    );

    let elapsed_time = time.elapsed();
    println!("Done, run took {} seconds.", elapsed_time.as_secs_f32());
}
