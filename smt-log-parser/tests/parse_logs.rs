use std::time::{Duration, Instant};

use smt_log_parser::{LogParser, Z3Parser};

#[test]
fn parse_all_logs() {
    std::env::set_var("SLP_TEST_MODE", "true");

    let all_logs = std::fs::read_dir("../logs").unwrap();
    for log in all_logs {
        let filename = log.unwrap().path();
        let (metadata, mut parser) = Z3Parser::from_file(&filename).unwrap();
        let file_size = metadata.len();

        // Gives 50 millis per MB (or 50 secs per GB)
        let timeout = Duration::from_millis(500 + (file_size / 20_000));
        println!("Parsing {} ({} MB) with timeout of {timeout:?}", filename.display(), file_size / 1024 / 1024);
        let now = Instant::now();

        let _ = parser.process_check_every(Duration::from_millis(100), |_, _| {
            assert!(now.elapsed() < timeout, "Parsing took longer than timeout");
            let physical_mem = memory_stats::memory_stats().unwrap().physical_mem as u64;
            assert!(physical_mem < file_size, "Memory usage was {} MB, but file size was {} MB", physical_mem / 1024 / 1024, file_size / 1024 / 1024);
            true
        }).unwrap();
        drop(parser);
    }
}
