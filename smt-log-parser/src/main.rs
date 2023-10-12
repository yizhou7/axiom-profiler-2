use prototype::interrupter::ParseInterrupter;
use prototype::file_io::*;
use prototype::parsers::*;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = get_settings();
    let filename = if args.len() < 2 {
        settings.file.to_string()
    } else {
        args[1].to_string()
    };
    let mut parser = z3parser1::new();
    let mut interrupt_agent = ParseInterrupter::new(&parser);
    if settings.timeout > 0.0 {
        let _timer = thread::spawn(move || {
            thread::sleep(Duration::from_secs_f32(10.0));
            interrupt_agent.stop_parsing();
        });
        let read_parse = thread::spawn(move || {
            let settings = settings.clone();
            if let Err(e) = parser.process_z3_file(&filename, &settings) {
                println!("{}", e);
            }
        });
        if let Err(e) = read_parse.join() {
            println!("{:?}", e);
        };
    } else if let Err(e) = parser.process_z3_file(&filename, &settings) {
        println!("{}", e);
    }
}
