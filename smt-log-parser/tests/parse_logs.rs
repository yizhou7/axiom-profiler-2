use std::time::Duration;

use smt_log_parser::{LogParser, Z3Parser};

#[test]
fn parse_all_logs() {
    std::env::set_var("SLP_TEST_MODE", "true");

    let all_logs = std::fs::read_dir("../logs").unwrap();
    for log in all_logs {
        let filename = log.unwrap().path();
        let (metadata, parser) = Z3Parser::from_file(&filename).unwrap();
        // Set limit if supported
        let _ = rlimit::Resource::AS.set(metadata.len() * 4, rlimit::Resource::AS.get_hard().unwrap_or(rlimit::INFINITY));

        // Gives 50 millis per MB (or 50 secs per GB)
        let to = Duration::from_millis(500 + (metadata.len() / 20_000));

        println!("Parsing {} ({} MB) with timeout of {to:?}", filename.display(), metadata.len() / 1024 / 1024);
        let (timeout, result) = parser.process_all_timeout(to);

        assert!(timeout.is_none());
        let physical_mem = memory_stats::memory_stats().unwrap().physical_mem as u64;
        println!("Have {} MB of physical memory",physical_mem / 1024 / 1024);
        assert!(physical_mem < metadata.len(), "Memory usage was {} MB, but file size was {} MB", physical_mem / 1024 / 1024, metadata.len() / 1024 / 1024);
        drop(result);
    }
}
