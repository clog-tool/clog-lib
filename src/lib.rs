// DOCS

extern crate indexmap;
extern crate regex;
extern crate time;
extern crate toml;

#[macro_use]
mod macros;
mod clog;
pub mod error;
pub mod fmt;
pub mod git;
mod link_style;
mod sectionmap;

pub use clog::Clog;
pub use link_style::LinkStyle;
pub use sectionmap::SectionMap;

// The default config file
const CLOG_CONFIG_FILE: &str = ".clog.toml";
