use std::{fmt::Debug, str::Split};

use super::LogParser;

/// Original Z3 log parser. Works with Z3 v.4.12.1, should work with other versions
/// as long as the log format is the same for the important line cases.
/// Compare with the log files in the `logs/` folder to see if this is the case.
pub mod z3parser;
// pub mod dump;

impl<T: Z3LogParser + Default> LogParser for T {
    fn process_line(&mut self, line: &str, line_no: usize) -> bool {
        // println!("Processing {line:?} ({line_no})");
        let mut split = line.split(' ');
        let parse = match split.next().unwrap() {
            // match the line case
            "[tool-version]" => self.version_info(split),
            "[mk-quant]" | "[mk-lambda]" => self.mk_quant(split),
            "[mk-var]" => self.mk_var(split),
            "[mk-proof]" => self.mk_proof_app(split, true),
            "[mk-app]" => self.mk_proof_app(split, false),
            "[attach-meaning]" => self.attach_meaning(split),
            "[attach-var-names]" => self.attach_var_names(split),
            "[attach-enode]" => self.attach_enode(split),
            "[eq-expl]" => self.eq_expl(split),
            "[new-match]" => self.new_match(split, line_no),
            "[inst-discovered]" => self.inst_discovered(split, line_no),
            "[instance]" => self.instance(split, line_no),
            "[end-of-instance]" => {
                self.end_of_instance();
                Some(())
            }
            "[decide-and-or]" => self.decide_and_or(split),
            "[decide]" => self.decide(split),
            "[assign]" => self.assign(split),
            "[push]" => self.push(split),
            "[pop]" => self.pop(split),
            "[begin-check]" => self.begin_check(split),
            "[query-done]" => self.query_done(split),
            "[eof]" => return false,
            "[resolve-process]" => self.resolve_process(split),
            "[resolve-lit]" => self.resolve_lit(split),
            "[conflict]" => self.conflict(split),
            _ => None,
        };
        parse.unwrap_or_else(|| eprintln!("Error parsing line: {line:?}"));
        true
    }
}

const DEFAULT: Option<()> = Some(());
pub trait Z3LogParser: Debug {
    /* Methods to handle each line case of Z3 logs.
     `l` is a line split with spaces as delimiters,
     and `l0` is the raw line (used only when )
    */
    fn version_info(&mut self, l: Split<'_, char>) -> Option<()>;
    fn mk_quant(&mut self, l: Split<'_, char>) -> Option<()>;
    fn mk_var(&mut self, l: Split<'_, char>) -> Option<()>;
    fn mk_proof_app(&mut self, l: Split<'_, char>, is_proof: bool) -> Option<()>;
    fn attach_meaning(&mut self, l: Split<'_, char>) -> Option<()>;
    fn attach_var_names(&mut self, l: Split<'_, char>) -> Option<()>;
    fn attach_enode(&mut self, l: Split<'_, char>) -> Option<()>;
    fn eq_expl(&mut self, l: Split<'_, char>) -> Option<()>;
    fn new_match(&mut self, l: Split<'_, char>, line_no: usize) -> Option<()>;
    fn inst_discovered(&mut self, l: Split<'_, char>, line_no: usize) -> Option<()>;
    fn instance(&mut self, l: Split<'_, char>, line_no: usize) -> Option<()>;
    fn end_of_instance(&mut self);

    // unused in original parser
    fn decide_and_or(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn decide(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn assign(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn push(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn pop(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn begin_check(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn query_done(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn resolve_process(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn resolve_lit(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
    fn conflict(&mut self, _l: Split<'_, char>) -> Option<()> {
        DEFAULT
    }
}

/// Type of solver and version number
#[derive(Debug)]
pub struct VersionInfo {
    solver: String,
    version: semver::Version,
}
impl VersionInfo {
    pub fn solver(&self) -> &str {
        &self.solver
    }
    pub fn version(&self) -> &semver::Version {
        &self.version
    }
}
