use std::time::Duration;

use smt_log_parser::{LogParser, Z3Parser};

#[test]
fn parse_all_logs() {
    std::env::set_var("SLP_TEST_MODE", "true");

    let all_logs = std::fs::read_dir("../logs").unwrap();
    for log in all_logs {
        let filename = log.unwrap().path();
        println!("Parsing {}", filename.display());
        let (metadata, parser) = Z3Parser::from_file(filename).unwrap();
        // Gives 10 millis per MB (or 10 secs per GB)
        let to = Duration::from_millis(500 + (metadata.len() / 100_000));
        let (timeout, _result) = parser.process_all_timeout(to);
        assert!(timeout.is_none());
    }
}
