// Until regex_macros compiles on nightly, we comment this out
//
// #![cfg_attr(feature = "unstable", feature(plugin))]
// #![cfg_attr(feature = "unstable", plugin(regex_macros))]

// DOCS

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
