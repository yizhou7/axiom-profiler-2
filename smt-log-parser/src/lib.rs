/// Commonly used file reading/writing sequences.
pub mod file_io;

/// Parsing items, e.g. terms, proof steps, quantifiers; and related objects or functions.
pub mod items;

/// Rendering for quantifier instantiation graphs.
mod render;

/// Generates CSS style sheets for static SVG outputs.
mod css;

/// Generates Dot output from instantiation information.
mod dot_output;

/// Manages different sorting/filtering functionality (currently empty).
mod sort_filter;

/// Parser structs and methods.
pub mod parsers;

/// Early stopping functionality for parsers.
pub mod interrupter;
