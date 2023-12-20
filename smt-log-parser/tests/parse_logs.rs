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
        let _ = rlimit::Resource::AS.set(metadata.len(), rlimit::Resource::AS.get_hard().unwrap_or(rlimit::INFINITY)).unwrap();
        // Gives 50 millis per MB (or 50 secs per GB)
        // let to = Duration::from_millis(500 + (metadata.len() / 20_000));
        let to = Duration::from_secs(600);
        println!("Parsing {} ({} MB) with timeout of {to:?}", filename.display(), metadata.len() / 1024 / 1024);
        let (timeout, result) = parser.process_all_timeout(to);
        assert!(timeout.is_none());
        drop(result);
    }
}
