use std::fmt::Debug;
use std::fs::{File, Metadata};
use std::path::Path;
use futures::{AsyncRead, AsyncBufRead, AsyncBufReadExt};
use wasm_timer::Instant;
use std::time::Duration;
use std::io::{BufRead, BufReader, Read};

use self::private::{AsyncParser, StreamParser};

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
    fn from_buf<R: BufRead>(reader: R) -> StreamParser<R, Self> {
        StreamParser::new(reader)
    }
    fn from_async_buf<R: AsyncBufRead + Unpin>(reader: R) -> AsyncParser<R, Self> {
        AsyncParser::new(reader)
    }

    /// Creates a new parser from the contents of a log file. The `end_data` is the total
    /// number of bytes in the log.
    fn from_str<R: Read, S: IntoStreamParser<R>>(ts: S) -> StreamParser<R, Self> {
        ts.smt_parser_from()
    }
    /// Creates a new streaming parser from a file. The `end_data` is the total
    /// number of bytes in the file.
    /// 
    /// This method is an alternative to `from_str(fs::read_to_string(self)?)`.
    /// This approach to parsing is ~5% slower, but should use only ~50% as much
    /// memory due to not having the entire loaded String in memory.
    fn from_file<R: Read, Meta, Error, S: IntoStreamParserMaybe<R, Meta, Error>>(ts: S)
            -> Result<(Meta, StreamParser<R, Self>), Error> {
        ts.smt_try_parser_from()
    }
    fn from_async<R: AsyncRead, S: IntoAsyncParser<R>>(ts: S) -> AsyncParser<R, Self> {
        ts.smt_async_parser_from()
    }
}

////////////////////
// Parser Creation 
////////////////////

pub trait IntoStreamParser<R: Read> {
    /// TODO: doc
    fn smt_parser_from<Parser: LogParser>(self) -> StreamParser<R, Parser>;
}
impl<'a> IntoStreamParser<&'a [u8]> for &'a str {
    fn smt_parser_from<Parser: LogParser>(self) -> StreamParser<&'a [u8], Parser> {
        Parser::from_buf(self.as_bytes())
    }
}

pub trait IntoStreamParserMaybe<R: Read, Meta, Error> {
    /// Creates a new streaming parser from a file. The `end_data` is the total
    /// number of bytes in the file.
    /// 
    /// This method is an alternative to `fs::read_to_string(self)?.smt_parser_from()`.
    /// This approach to parsing is ~5% slower, but should use only ~50% as much
    /// memory due to not having the entire loaded String in memory.
    fn smt_try_parser_from<Parser: LogParser>(self) -> Result<(Meta, StreamParser<R, Parser>), Error>;
}
impl<P: AsRef<Path>> IntoStreamParserMaybe<BufReader<File>, Metadata, std::io::Error> for P {
    fn smt_try_parser_from<Parser: LogParser>(self) -> std::io::Result<(Metadata, StreamParser<BufReader<File>, Parser>)> {
        let file = File::open(self)?;
        let metadata = file.metadata()?;
        let reader = BufReader::new(file);
        Ok((metadata, Parser::from_buf(reader)))
    }
}

pub trait IntoAsyncParser<R: AsyncRead> {
    /// TODO: doc
    fn smt_async_parser_from<Parser: LogParser>(self) -> AsyncParser<R, Parser>;
}
impl<R: AsyncRead + Unpin> IntoAsyncParser<futures::io::BufReader<R>> for R {
    fn smt_async_parser_from<Parser: LogParser>(self) -> AsyncParser<futures::io::BufReader<R>, Parser> {
        let reader = futures::io::BufReader::new(self);
        Parser::from_async_buf(reader)
    }
}
impl<'a> IntoAsyncParser<&'a [u8]> for &'a str {
    fn smt_async_parser_from<Parser: LogParser>(self) -> AsyncParser<&'a [u8], Parser> {
        Parser::from_async_buf(self.as_bytes())
    }
}

////////////////////
// Parser Execution
////////////////////

/// Hides [`StreamParser`] and [`AsyncParser`] from the public API. Use
/// [`LogParser::from_buf`] or [`LogParser::from_async_buf`] to create a parser.
mod private {
    use super::*;

    #[duplicate::duplicate_item(
        EitherParser   ReadBound;
        [StreamParser] [Read];
        [AsyncParser]  [AsyncRead];
    )]
    /// Struct for a generic SMT solver trace parser. Supports parsing line-by-line with a callback to
    /// indicate if or when to pause. Intended to support different solvers or log formats.
    pub struct EitherParser<R: ReadBound, P: LogParser> {
        pub(super) reader: R,
        pub(super) reader_state: ReaderState,
        pub(super) parser: P,
        pub(super) finished: bool,
    }
    
    #[duplicate::duplicate_item(
        EitherParser   ReadBound              async   add_await(code);
        [StreamParser] [BufRead]              []      [code];
        [AsyncParser]  [AsyncBufRead + Unpin] [async] [code.await];
    )]
    impl<R: ReadBound, Parser: LogParser> EitherParser<R, Parser> {
        pub(super) fn new(reader: R) -> Self {
            Self { reader, reader_state: ReaderState::default(), parser: Parser::default(), finished: false }
        }
        pub async fn process_until<'a>(&'a mut self, stop: impl Fn(&Parser, &ReaderState) -> bool) -> ProcessResult<'a, Parser, ReaderState> {
            assert!(!self.finished);
            let mut buffer = String::new();
            while !stop(&self.parser, &self.reader_state) {
                buffer.clear();
                let bytes_read = add_await([self.reader.read_line(&mut buffer)]).unwrap();
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
        pub async fn process_for<'a>(&'a mut self, time: Duration) -> ProcessResult<'a, Parser, ReaderState> {
            // Will actually process for up to (ms_per_line * 100)% longer than `time`, thus
            // we are assuming that we can process lines faster than 0.1ms per line.
            let check_freq = time.as_millis().try_into().unwrap_or(usize::MAX);
            let start = Instant::now();
            add_await([self.process_until(
                |_, state| state.lines_read % check_freq == 0 && Instant::now() - start > time
            )])
        }
    
        /// Parse the entire file as a stream. Using [`process_all_timeout`] instead is
        /// recommended as this method will cause the process to hang if given a very large file.
        pub async fn process_all(mut self) -> Parser {
            let ProcessResult::Return(result) = add_await([self.process_until(|_, _| false)]) else {
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
        pub async fn process_all_timeout(mut self, timeout: Duration) -> (bool, Parser) {
            let result = add_await([self.process_for(timeout)]);
            let result = result.return_state();
            (result.is_none(), result.unwrap_or(self.parser))
        }
    }
}

////////////////////
// Parser Output
////////////////////

/// The result of a single call to [`process_until`].
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ReaderState {
    /// The number of bytes parsed so far.
    pub bytes_read: usize,
    /// The number of lines parsed so far.
    pub lines_read: usize,
}
