use futures::{AsyncBufRead, AsyncBufReadExt, AsyncRead};
use std::fmt::Debug;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, Result};
use std::path::Path;
use std::time::Duration;
use wasm_timer::Instant;
pub use self::wrapper_stream_parser::*;
pub use self::wrapper_async_parser::*;

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
    fn from<'r, R: BufRead + 'r>(reader: R) -> StreamParser<'r, Self> {
        StreamParser::new(reader)
    }
    fn from_async<'r, R: AsyncBufRead + Unpin + 'r>(reader: R) -> AsyncParser<'r, Self> {
        AsyncParser::new(reader)
    }

    /// Creates a new parser from the contents of a log file.
    fn from_str<'r>(s: &'r str) -> StreamParser<'r, Self> {
        s.as_bytes().into_parser()
    }
    /// Creates a new parser from the contents of a log file.
    fn from_string(s: String) -> StreamParser<'static, Self> {
        s.into_cursor().into_parser()
    }

    /// Creates a new streaming parser from a file. The `end_data` is the total
    /// number of bytes in the file.
    ///
    /// This method is an alternative to `from_string(fs::read_to_string(self)?)`.
    /// This approach to parsing is ~5% slower, but should use only ~50% as much
    /// memory due to not having the entire loaded String in memory.
    fn from_file<P: AsRef<Path>>(
        p: P,
    ) -> Result<(Metadata, StreamParser<'static, Self>)> {
        let (meta, reader) = p.read_open()?;
        Ok((meta, reader.into_parser()))
    }
}

////////////////////
// Parser Creation
////////////////////

pub trait IntoStreamParser<'r> {
    /// TODO: doc
    fn into_parser<Parser: LogParser>(self) -> StreamParser<'r, Parser>;
}
impl<'r, R: BufRead + 'r> IntoStreamParser<'r> for R {
    fn into_parser<Parser: LogParser>(self) -> StreamParser<'r, Parser> {
        Parser::from(self)
    }
}

pub trait CursorRead: AsRef<[u8]> + Sized {
    /// Turns any `[u8]` data such as a [`String`] into
    /// a [`Cursor`](std::io::Cursor) which implements [`BufRead`](std::io::BufRead).
    /// Intended to be chained with [`into_parser`](IntoStreamParser::into_parser).
    fn into_cursor(self) -> std::io::Cursor<Self> {
        std::io::Cursor::new(self)
    }
}
impl<T: AsRef<[u8]>> CursorRead for T {}

pub trait FileRead: AsRef<Path> + Sized {
    fn read_open(self) -> Result<(Metadata, BufReader<File>)> {
        let file = File::open(self)?;
        let metadata = file.metadata()?;
        let reader = BufReader::new(file);
        Ok((metadata, reader))
    }
}
impl<T: AsRef<Path>> FileRead for T {}

pub trait IntoAsyncParser<'r> {
    /// TODO: doc
    fn into_async_parser<Parser: LogParser>(self) -> AsyncParser<'r, Parser>;
}
impl<'r, R: AsyncBufRead + Unpin + 'r> IntoAsyncParser<'r> for R {
    fn into_async_parser<Parser: LogParser>(
        self,
    ) -> AsyncParser<'r, Parser> {
        Parser::from_async(self)
    }
}

pub trait AsyncBufferRead: AsyncRead + Sized {
    fn buffer(self) -> futures::io::BufReader<Self> {
        futures::io::BufReader::new(self)
    }
}
impl<T: AsyncRead> AsyncBufferRead for T {}

pub trait AsyncCursorRead: AsRef<[u8]> + Unpin + Sized {
    fn into_async_cursor(self) -> futures::io::Cursor<Self> {
        futures::io::Cursor::new(self)
    }
}
impl<T: AsRef<[u8]> + Unpin> AsyncCursorRead for T {}

////////////////////
// Parser Execution
////////////////////

#[derive(Debug, Clone, Copy, Default)]
pub struct ReaderState {
    /// The number of bytes parsed so far.
    pub bytes_read: usize,
    /// The number of lines parsed so far.
    pub lines_read: usize,
}

#[duplicate::duplicate_item(
    EitherParser   ReadBound              async   add_await(code);
    [StreamParser] [BufRead + 'r]              []      [code];
    [AsyncParser]  [AsyncBufRead + Unpin + 'r] [async] [code.await];
)]
mod wrapper {
    use super::*;

    /// Struct for a generic SMT solver trace parser. Supports parsing line-by-line with a callback to
    /// indicate if or when to pause. Intended to support different solvers or log formats.
    pub struct EitherParser<'r, P: LogParser> {
        reader: Box<dyn ReadBound>,
        reader_state: ReaderState,
        parser: Option<P>,
    }
    impl<'r, P: LogParser, R: ReadBound> From<R> for EitherParser<'r, P> {
        fn from(reader: R) -> Self {
            Self::new(reader)
        }
    }
    impl<'r, Parser: LogParser> EitherParser<'r, Parser> {
        pub(super) fn new(reader: impl ReadBound) -> Self {
            Self {
                reader: Box::new(reader),
                reader_state: ReaderState::default(),
                parser: Some(Parser::default()),
            }
        }

        pub fn parser(&self) -> Option<&Parser> {
            self.parser.as_ref()
        }
        pub fn reader_state(&self) -> &ReaderState {
            &self.reader_state
        }
        pub async fn process_until<'a>(
            &'a mut self,
            stop: impl Fn(&Parser, &ReaderState) -> bool,
        ) -> Option<Parser> {
            let Some(parser) = self.parser.as_mut() else {
                return None;
            };
            let mut buffer = String::new();
            while !stop(&*parser, &self.reader_state) {
                buffer.clear();
                let bytes_read = add_await([self.reader.read_line(&mut buffer)]).unwrap();
                if bytes_read == 0
                    || !parser
                        .process_line(buffer.trim_end(), self.reader_state.lines_read)
                {
                    return self.parser.take();
                }
                self.reader_state.bytes_read += bytes_read;
                self.reader_state.lines_read += 1;
            }
            None
        }
        pub async fn process_for<'a>(
            &'a mut self,
            time: Duration,
        ) -> Option<Parser> {
            // Will actually process for up to (ms_per_line * 100)% longer than `time`, thus
            // we are assuming that we can process lines faster than 0.1ms per line.
            let check_freq = time.as_millis().try_into().unwrap_or(usize::MAX);
            let start = Instant::now();
            add_await([self.process_until(|_, state| {
                state.lines_read % check_freq == 0 && Instant::now() - start > time
            })])
        }
    
        /// Parse the entire file as a stream. Using [`process_all_timeout`] instead is
        /// recommended as this method will cause the process to hang if given a very large file.
        pub async fn process_all(mut self) -> Parser {
            add_await([self.process_until(|_, _| false)]).unwrap()
        }
        /// Try to parse everything, but stop after a given timeout. The result tuple
        /// contains `true` if the timeout was reached, and the parser state at the end (i.e. the
        /// state is complete only if `false` was returned).
        ///
        /// Parsing cannot be resumed if the timeout is reached. If you need support for resuming,
        /// use [`process_for`] or [`process_until`] instead.
        pub async fn process_all_timeout(mut self, timeout: Duration) -> (bool, Parser) {
            let result = add_await([self.process_for(timeout)]);
            (result.is_none(), result.unwrap_or_else(|| self.parser.unwrap()))
        }
    }
}
