use std::fs::{File, OpenOptions, self};
use std::io::{self, BufRead, Write, BufWriter};
use std::path::Path;
use serde::Deserialize;

use crate::items;

/// settings file
const SETTINGS: &str = "settings.json";
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
pub fn write<T>(file: &mut BufWriter<File>, obj: &T) where T: items::Print {
    file.write_all(obj.format().as_bytes()).expect("write failed");
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

/// Parsing settings.
#[derive(Default, Clone, Debug, Deserialize)]
pub struct Settings {
    // not all settings currently work
    /// Name of file to parse.
    pub file: String,
    /// Whether to consider terms with reused IDs as separate from previous terms with the same ID. Does not work yet.
    pub reuses: bool,
    /// Print contents of parser to standard output item by item.
    pub verbose: bool,
    /// Whether to print all text/json files (Dot and SVG will always be generated regardless of this setting)
    pub save_all_data: bool,
    /// Select a sort type (by line number, cost, depth, etc.)
    /// Does not work yet.
    /// ## TODO
    /// Replace with an enum
    pub sort_by: String,
    /// Stop parsing after a certain amount of time. Does not work yet.
    pub timeout: f32,
    /// Parse only up to a certain number of lines. Does not work yet.
    pub line_limit: usize,
    // add settings for:
    // - number of instantiations to display in final visualization.
}


/// Read settings from `SETTINGS` json file, saving them to a `Settings` struct
pub fn get_settings() -> Settings {
    let settings_text = fs::read_to_string(SETTINGS).expect("settings file should exist");
    serde_json::from_str(&settings_text).expect("Settings file should be valid JSON")
}