// DOCS

extern crate indexmap;
extern crate regex;
extern crate toml;
extern crate time;

#[macro_use]
mod macros;
pub mod git;
pub mod fmt;
mod sectionmap;
mod clog;
pub mod error;
mod link_style;

pub use clog::Clog;
pub use sectionmap::SectionMap;
pub use link_style::LinkStyle;

// The default config file
const CLOG_CONFIG_FILE: &'static str = ".clog.toml";
