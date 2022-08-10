use std::str::FromStr;

use strum::{Display, EnumString};

/// Determines the hyperlink style used in commit and issue links. Defaults to
/// `LinksStyle::Github`
///
/// # Example
///
/// ```no_run
/// # use clog::{LinkStyle, Clog};
/// let clog = Clog::new().unwrap();
/// clog.link_style(LinkStyle::Stash);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Display, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum LinkStyle {
    Github,
    Gitlab,
    Stash,
    Cgit,
}

impl Default for LinkStyle {
    fn default() -> Self { LinkStyle::Github }
}

impl<'de> serde::de::Deserialize<'de> for LinkStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl LinkStyle {
    /// Gets a hyperlink url to an issue in the specified format.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::{LinkStyle, Clog};
    /// let link = LinkStyle::Github;
    /// let issue = link.issue_link("141", Some("https://github.com/thoughtram/clog"));
    ///
    /// assert_eq!("https://github.com/thoughtram/clog/issues/141", issue);
    /// ```
    pub fn issue_link<S: AsRef<str>>(&self, issue: S, repo: Option<S>) -> String {
        let issue = issue.as_ref();
        if let Some(link) = repo {
            let link = link.as_ref();
            match *self {
                LinkStyle::Github | LinkStyle::Gitlab => format!("{link}/issues/{issue}"),
                // cgit does not support issues
                LinkStyle::Stash | LinkStyle::Cgit => issue.to_string(),
            }
        } else {
            issue.to_string()
        }
    }

    /// Gets a hyperlink url to a commit in the specified format.
    ///
    /// # Example
    /// ```no_run
    /// # use clog::{LinkStyle, Clog};
    /// let link = LinkStyle::Github;
    /// let commit = link.commit_link(
    ///     "123abc891234567890abcdefabc4567898724",
    ///     Some("https://github.com/clog-tool/clog-lib"),
    /// );
    ///
    /// assert_eq!(
    ///     "https://github.com/thoughtram/clog/commit/123abc891234567890abcdefabc4567898724",
    ///     commit
    /// );
    /// ```
    pub fn commit_link<S: AsRef<str>>(&self, hash: S, repo: Option<S>) -> String {
        let hash = hash.as_ref();
        if let Some(link) = repo {
            let link = link.as_ref();
            match *self {
                LinkStyle::Github | LinkStyle::Gitlab => format!("{link}/commit/{hash}"),
                LinkStyle::Stash => format!("{link}/commits/{hash}"),
                LinkStyle::Cgit => format!("{link}/commit/?id={hash}"),
            }
        } else {
            (hash[0..8]).to_string()
        }
    }
}
