use std::{path::PathBuf, result::Result as StdResult};

use thiserror::Error;

pub type Result<T> = StdResult<T, Error>;

/// An enum for describing and handling various errors encountered while dealing
/// with `clog` building, or writing of changelogs.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Found unknown component '{0}' that does not correspond to a Changelog Section")]
    UnknownComponent(String),

    #[error("failed to parse config file: {0}")]
    ConfigParse(PathBuf),

    #[error("incorrect format for config file: {0}")]
    ConfigFormat(PathBuf),

    #[error("cannot get current directory")]
    CurrentDir,

    #[error("unrecognized link-style field")]
    LinkStyle,

    #[error("fatal I/O error with output file")]
    Io(#[from] std::io::Error),

    #[error("failed to convert date/time to string format")]
    TimeFormat(#[from] time::error::Format),

    #[error("failed to convert date/time to string format")]
    Time(#[from] time::Error),

    #[error("failed to convert {0} to valid ChangelogFormat")]
    ChangelogFormat(String),

    #[error("Failed to parse TOML configuration file")]
    Toml(#[from] toml::de::Error),

    #[error("unknown fatal error")]
    Unknown,
}
