use std::fmt::Display;
use std::fs::{File, OpenOptions, self};
use std::io::{self, BufRead, Write, BufWriter};
use std::path::Path;
use serde::Deserialize;

/// hard-coded write buffer capacity
const CAPACITY: usize = 1 << 25; // 32 MiB

/// Returns an iterator for the lines of text in a text file.
/// Used to avoid reading entire file at once.
/// # Errors
/// Errors if file cannot be opened.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Writes the debug text of `obj` to `file`.
/// # Panics
/// Panics if an error occurs when writing to `file`.
pub fn write<T>(file: &mut BufWriter<File>, obj: &T) where T: Display {
    file.write_all(obj.to_string().as_bytes()).expect("write failed");
}

/// Writes `text` to `file`. More general-purpose than `write<T>`.
/// # Panics
/// Panics if an error occurs when writing to `file`.
pub fn write_str(file: &mut BufWriter<File>, text: &str) {
    file.write_all(text.as_bytes()).expect("write failed");
}

/// Returns a file to write to after creating it (if necessary) and truncating it.
/// # Panics
/// Panics if an error occurs when opening file.
pub fn open_file_truncate(filename: &str) -> BufWriter<File> {
    BufWriter::with_capacity(CAPACITY, OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(filename).unwrap_or_else(|_| panic!("Error opening {}", filename)))
}
