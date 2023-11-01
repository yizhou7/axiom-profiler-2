use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use futures::{AsyncRead, AsyncBufReadExt};
use wasm_timer::Instant;
use std::time::Duration;
use std::io::{BufRead, BufReader, Read};

pub mod z3;

// TODO: use generators once stabilized
pub enum ProcessResult<'a, T> {
    Yield(&'a T),
    Return(T),
}

impl<'a, T> ProcessResult<'a, T> {
    pub fn as_return(self) -> Option<T> {
        match self {
            ProcessResult::Yield(..) => None,
            ProcessResult::Return(result) => Some(result),
        }
    }
}

/// Struct for a generic SMT solver trace parser. Supports parsing line-by-line with a callback to
/// indicate if or when to pause. Intended to support different solvers or log formats.
pub struct StreamParser<R: Read, P: LogParser> {
    reader: R,
    line_no: usize,
    parser: P,
    finished: bool,
}
impl<R: BufRead, Parser: LogParser> StreamParser<R, Parser> {
    pub fn new(reader: R) -> Self {
        Self { reader, line_no: 0, parser: Parser::default(), finished: false }
    }
    pub fn process_until<'a>(&'a mut self, stop: impl Fn(&Parser, usize) -> bool) -> ProcessResult<'a, Parser> {
        assert!(!self.finished);
        let mut buffer = String::new();
        while !stop(&self.parser, self.line_no) {
            buffer.clear();
            let bytes_read = self.reader.read_line(&mut buffer).unwrap();
            if bytes_read == 0 || !self.parser.process_line(buffer.trim_end(), self.line_no) {
                self.finished = true;
                let parser = std::mem::replace(&mut self.parser, Parser::default());
                return ProcessResult::Return(parser);
            }
            self.line_no += 1;
        }
        ProcessResult::Yield(&self.parser)
    }
    pub fn process_all(&mut self) -> Parser {
        let ProcessResult::Return(result) = self.process_until(|_, _| false) else {
            unreachable!()
        };
        result
    }
}
impl<Parser: LogParser> StreamParser<BufReader<File>, Parser> {
    pub fn new_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        Ok(Self::new(reader))
    }
    pub fn parse_entire_file<P: AsRef<Path>>(path: P, timeout: Duration) -> std::io::Result<(bool, Parser)> {
        let mut this = Self::new_file(path)?;
        let time = Instant::now();
        let result = this.process_until(|_, _| !timeout.is_zero() && Instant::now() - time > timeout);
        let result = result.as_return();
        Ok((result.is_none(), result.unwrap_or(this.parser)))
    }
}
impl<'a, Parser: LogParser> StreamParser<&'a [u8], Parser> {
    pub fn new_string(file: &'a str) -> Self {
        Self::new(file.as_bytes())
    }
    pub fn parse_entire_string(string: &'a str, timeout: Duration) -> (bool, Parser) {
        let mut this = Self::new_string(string);
        let time = Instant::now();
        let result = this.process_until(|_, _| !timeout.is_zero() && Instant::now() - time > timeout);
        let result = result.as_return();
        (result.is_none(), result.unwrap_or(this.parser))
    }
}

/// Trait for a generic SMT solver trace parser. Intended to support different solvers or log formats. 
pub trait LogParser: Default + Debug {
    fn process_line(&mut self, line: &str, line_no: usize) -> bool;
}

/// Struct for a generic SMT solver trace parser. Intended to support different solvers or log formats.
pub struct AsyncStreamParser<R: AsyncRead, P: LogParser> {
    reader: R,
    line_no: usize,
    parser: P,
    finished: bool,
}
impl<R: AsyncBufReadExt + Unpin, Parser: LogParser> AsyncStreamParser<R, Parser> {
    pub fn new(reader: R) -> Self {
        Self { reader, line_no: 0, parser: Parser::default(), finished: false }
    }
    pub async fn process_until<'a>(&'a mut self, stop: impl Fn(&Parser, usize) -> bool) -> ProcessResult<'a, Parser> {
        assert!(!self.finished);
        let mut buffer = String::new();
        while !stop(&self.parser, self.line_no) {
            buffer.clear();
            let _bytes_read = self.reader.read_line(&mut buffer).await.unwrap();
            if !self.parser.process_line(buffer.trim_end_matches('\n'), self.line_no) {
                self.finished = true;
                let parser = std::mem::replace(&mut self.parser, Parser::default());
                return ProcessResult::Return(parser);
            }
        }
        ProcessResult::Yield(&self.parser)
    }
    pub async fn process_all(&mut self) -> Parser {
        let ProcessResult::Return(result) = self.process_until(|_, _| false).await else {
            unreachable!()
        };
        result
    }
}
impl<R: AsyncRead + Unpin, Parser: LogParser> AsyncStreamParser<futures::io::BufReader<R>, Parser> {
    pub fn new_read(reader: R) -> Self {
        let reader = futures::io::BufReader::new(reader);
        Self::new(reader)
    }
}
impl<'a, Parser: LogParser> AsyncStreamParser<&'a [u8], Parser> {
    pub fn new_string(file: &'a str) -> Self {
        Self { reader: file.as_bytes(), line_no: 0, parser: Parser::default(), finished: false }
    }
}
