#![doc = include_str!("../README.md")]

#[macro_use]
mod macros;
mod clog;
mod config;
pub mod error;
pub mod fmt;
pub mod git;
mod link_style;
mod sectionmap;

pub use crate::{clog::Clog, link_style::LinkStyle, sectionmap::SectionMap};

// The default config file
const DEFAULT_CONFIG_FILE: &str = ".clog.toml";
