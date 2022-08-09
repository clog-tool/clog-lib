mod json_writer;
mod md_writer;

use strum::{Display, EnumString};

pub use self::{json_writer::JsonWriter, md_writer::MarkdownWriter};
use crate::{clog::Clog, error::Result, sectionmap::SectionMap};

#[derive(Copy, Clone, PartialEq, Eq, Debug, EnumString, Display)]
#[strum(ascii_case_insensitive)]
pub enum ChangelogFormat {
    Json,
    Markdown,
}

/// A trait that allows writing the results of a `clog` run which can then be
/// written in an arbitrary format. The single required function
/// `write_changelog()` accepts a `clog::SectionMap` which can be thought of
/// similiar to a `clog` "AST" of sorts.
///
/// `clog` provides two default implementors of this traint,
/// `clog::fmt::MarkdownWriter` and `clog::fmt::JsonWriter` for writing Markdown
/// and JSON respectively
pub trait FormatWriter {
    /// Writes a changelog from a given `clog::SectionMap` which can be thought
    /// of as an "AST" of sorts
    fn write_changelog(&mut self, options: &Clog, section_map: &SectionMap) -> Result<()>;
}
