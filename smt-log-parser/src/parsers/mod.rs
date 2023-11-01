use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use futures::{AsyncRead, AsyncBufReadExt};
use wasm_timer::Instant;
use std::time::Duration;
use std::io::{BufRead, BufReader, Read};

pub mod z3;

/// Trait for a generic SMT solver trace parser. Intended to support different solvers or log formats. 
pub trait LogParser: Default + Debug {
    fn process_line(&mut self, line: &str, line_no: usize) -> bool;

    /// Creates a new parser. Only use this if you cannot use the following
    /// convenience methods:
    /// - [`new_file`] for creating a streaming parser from a file path
    /// - [`new_str`] for creating a parser from a strings
    /// 
    /// The provided `end_data` will be seen with each callback to `stop` in
    /// [`process_until`].
    fn from<R: BufRead, E>(reader: R, end_data: E) -> StreamParser<R, E, Self> {
        let reader_state = ReaderState::new(end_data);
        StreamParser { reader, reader_state, parser: Self::default(), finished: false }
    }
    fn from_str<R: Read, E, ToStream: ToStreamParser<R, E>>(ts: ToStream) -> StreamParser<R, E, Self> {
        ts.smt_parser_from()
    }
    /// Creates a new streaming parser from a file. The `end_data` is the total
    /// number of bytes in the file.
    /// 
    /// This method is an alternative to `fs::read_to_string(self)?.parser_from()`.
    /// This approach to parsing is ~5% slower, but should use only ~50% as much
    /// memory due to not having the entire loaded String in memory.
    fn from_file<R: Read, E, Error, ToStream: ToStreamParserMaybe<R, E, Error>>(ts: ToStream) -> Result<StreamParser<R, E, Self>, Error> {
        ts.smt_try_parser_from()
    }
}

pub trait ToStreamParser<R: Read, EndData> {
    /// TODO: doc
    fn smt_parser_from<Parser: LogParser>(self) -> StreamParser<R, EndData, Parser>;
}
impl<'a> ToStreamParser<&'a [u8], usize> for &'a str {
    fn smt_parser_from<Parser: LogParser>(self) -> StreamParser<&'a [u8], usize, Parser> {
        Parser::from(self.as_bytes(), self.len())
    }
}

pub trait ToStreamParserMaybe<R: Read, EndData, Error> {
    /// Creates a new streaming parser from a file. The `end_data` is the total
    /// number of bytes in the file.
    /// 
    /// This method is an alternative to `fs::read_to_string(self)?.smt_parser_from()`.
    /// This approach to parsing is ~5% slower, but should use only ~50% as much
    /// memory due to not having the entire loaded String in memory.
    fn smt_try_parser_from<Parser: LogParser>(self) -> Result<StreamParser<R, EndData, Parser>, Error>;
}
impl<P: AsRef<Path>> ToStreamParserMaybe<BufReader<File>, u64, std::io::Error> for P {
    fn smt_try_parser_from<Parser: LogParser>(self) -> std::io::Result<StreamParser<BufReader<File>, u64, Parser>> {
        let file = File::open(self)?;
        let bytes_size = file.metadata()?.len();
        let reader = BufReader::new(file);
        Ok(Parser::from(reader, bytes_size))
    }
}

/// The result of a single call to [`StreamParser::process_until`].
/// If the stopping condition is true then we yield the current parser state,
/// otherwise once the parser is finished we return the final parser state.
// TODO: use generators instead once they are stabilized
pub enum ProcessResult<'a, T, S> {
    Yield(&'a T, &'a S),
    Return(T),
}

impl<'a, T, S> ProcessResult<'a, T, S> {
    /// Return the parser state regardless of whether it is finished or not.
    pub fn yield_state(&self) -> Option<(&T, &S)> {
        match self {
            ProcessResult::Yield(result, rs) => Some((*result, *rs)),
            ProcessResult::Return(_) => None,
        }
    }
    /// Return the parser state regardless of whether it is finished or not.
    pub fn any_state(&self) -> &T {
        match self {
            ProcessResult::Yield(result, _) => *result,
            ProcessResult::Return(result) => result,
        }
    }
    /// Return the parser state if it is finished, otherwise return `None`.
    pub fn return_state(self) -> Option<T> {
        match self {
            ProcessResult::Yield(..) => None,
            ProcessResult::Return(result) => Some(result),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReaderState<E> {
    /// The number of bytes parsed so far.
    pub bytes_read: usize,
    /// The number of lines parsed so far.
    pub lines_read: usize,
    /// The end data provided when crating the parser
    /// state. Usually represents the total number of
    /// bytes needed to parse.
    pub end_data: E,
}
impl<E> ReaderState<E> {
    fn new(end_data: E) -> Self {
        Self { lines_read: 0, bytes_read: 0, end_data }
    }
}

/// Struct for a generic SMT solver trace parser. Supports parsing line-by-line with a callback to
/// indicate if or when to pause. Intended to support different solvers or log formats.
pub struct StreamParser<R: Read, E, P: LogParser> {
    reader: R,
    reader_state: ReaderState<E>,
    parser: P,
    finished: bool,
}

impl<R: BufRead, E, Parser: LogParser> StreamParser<R, E, Parser> {
    pub fn process_until<'a>(&'a mut self, stop: impl Fn(&Parser, &ReaderState<E>) -> bool) -> ProcessResult<'a, Parser, ReaderState<E>> {
        assert!(!self.finished);
        let mut buffer = String::new();
        while !stop(&self.parser, &self.reader_state) {
            buffer.clear();
            let bytes_read = self.reader.read_line(&mut buffer).unwrap();
            if bytes_read == 0 || !self.parser.process_line(buffer.trim_end(), self.reader_state.lines_read) {
                self.finished = true;
                let parser = std::mem::replace(&mut self.parser, Parser::default());
                return ProcessResult::Return(parser);
            }
            self.reader_state.bytes_read += bytes_read;
            self.reader_state.lines_read += 1;
        }
        ProcessResult::Yield(&self.parser, &self.reader_state)
    }
    pub fn process_for<'a>(&'a mut self, time: Duration) -> ProcessResult<'a, Parser, ReaderState<E>> {
        // Will actually process for up to (ms_per_line * 100)% longer than `time`, thus
        // we are assuming that we can process lines faster than 0.1ms per line.
        let check_freq = time.as_millis().try_into().unwrap_or(usize::MAX);
        let start = Instant::now();
        self.process_until(
            |_, state| state.lines_read % check_freq == 0 && Instant::now() - start > time
        )
    }

    /// Parse the entire file as a stream. Using [`process_all_timeout`] instead is
    /// recommended as this method will cause the process to hang if given a very large file.
    pub fn process_all(mut self) -> Parser {
        let ProcessResult::Return(result) = self.process_until(|_, _| false) else {
            unreachable!()
        };
        result
    }
    /// Try to parse everything, but stop after a given timeout. The result tuple
    /// contains `true` if the timeout was reached, and the parser state at the end (i.e. the
    /// state is complete only if `false` was returned).
    /// 
    /// Parsing cannot be resumed if the timeout is reached. If you need support for resuming,
    /// use [`process_for`] or [`process_until`] instead.
    pub fn process_all_timeout(mut self, timeout: Duration) -> (bool, Parser) {
        let result = self.process_for(timeout);
        let result = result.return_state();
        (result.is_none(), result.unwrap_or(self.parser))
    }
}

/// Struct for a generic SMT solver trace parser. Intended to support different solvers or log formats.
pub struct AsyncStreamParser<R: AsyncRead, E, P: LogParser> {
    reader: R,
    reader_state: ReaderState<E>,
    parser: P,
    finished: bool,
}
impl<R: AsyncBufReadExt + Unpin, E, Parser: LogParser> AsyncStreamParser<R, E, Parser> {
    pub fn new(reader: R, end_data: E) -> Self {
        let reader_state = ReaderState::new(end_data);
        Self { reader, reader_state, parser: Parser::default(), finished: false }
    }
    pub async fn process_until<'a>(&'a mut self, stop: impl Fn(&Parser, &ReaderState<E>) -> bool) -> ProcessResult<'a, Parser, ReaderState<E>> {
        assert!(!self.finished);
        let mut buffer = String::new();
        while !stop(&self.parser, &self.reader_state) {
            buffer.clear();
            let bytes_read = self.reader.read_line(&mut buffer).await.unwrap();
            if bytes_read == 0 || !self.parser.process_line(buffer.trim_end_matches('\n'), self.reader_state.lines_read) {
                self.finished = true;
                let parser = std::mem::replace(&mut self.parser, Parser::default());
                return ProcessResult::Return(parser);
            }
            self.reader_state.bytes_read += bytes_read;
            self.reader_state.lines_read += 1;
        }
        ProcessResult::Yield(&self.parser, &self.reader_state)
    }
    pub async fn process_all(&mut self) -> Parser {
        let ProcessResult::Return(result) = self.process_until(|_, _| false).await else {
            unreachable!()
        };
        result
    }
}
impl<R: AsyncRead + Unpin, Parser: LogParser> AsyncStreamParser<futures::io::BufReader<R>, (), Parser> {
    pub fn new_read(reader: R) -> Self {
        let reader = futures::io::BufReader::new(reader);
        Self::new(reader, ())
    }
}
impl<'a, Parser: LogParser> AsyncStreamParser<&'a [u8], usize, Parser> {
    pub fn new_string(file: &'a str) -> Self {
        Self::new(file.as_bytes(), file.len())
    }
}
