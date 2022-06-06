/// Determines the hyperlink style used in commit and issue links. Defaults to `LinksStyle::Github`
///
/// # Example
///
/// ```no_run
/// # use clog::{LinkStyle, Clog};
/// let mut clog = Clog::new().unwrap();
/// clog.link_style(LinkStyle::Stash);
/// ```
clog_enum! {
    #[derive(Debug)]
    pub enum LinkStyle {
        Github,
        Gitlab,
        Stash,
        Cgit
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
    /// let issue = link.issue_link("141", "https://github.com/thoughtram/clog");
    ///
    /// assert_eq!("https://github.com/thoughtram/clog/issues/141", issue);
    /// ```
    pub fn issue_link<S: AsRef<str>>(&self, issue: S, repo: S) -> String {
        match repo.as_ref() {
            "" => issue.as_ref().to_owned(),
            link => {
                match *self {
                    LinkStyle::Github => format!("{}/issues/{}", link, issue.as_ref()),
                    LinkStyle::Gitlab => format!("{}/issues/{}", link, issue.as_ref()),
                    LinkStyle::Stash => issue.as_ref().to_owned(),
                    // cgit does not support issues
                    LinkStyle::Cgit => issue.as_ref().to_owned(),
                }
            }
        }
    }

    /// Gets a hyperlink url to a commit in the specified format.
    ///
    /// # Example
    /// ```no_run
    /// # use clog::{LinkStyle, Clog};
    /// let link = LinkStyle::Github;
    /// let commit = link.commit_link("123abc891234567890abcdefabc4567898724", "https://github.com/thoughtram/clog");
    ///
    /// assert_eq!("https://github.com/thoughtram/clog/commit/123abc891234567890abcdefabc4567898724", commit);
    /// ```
    pub fn commit_link<S: AsRef<str>>(&self, hash: S, repo: S) -> String {
        match repo.as_ref() {
            "" => hash.as_ref()[0..8].to_string(),
            link => match *self {
                LinkStyle::Github => format!("{}/commit/{}", link, hash.as_ref()),
                LinkStyle::Gitlab => format!("{}/commit/{}", link, hash.as_ref()),
                LinkStyle::Stash => format!("{}/commits/{}", link, hash.as_ref()),
                LinkStyle::Cgit => format!("{}/commit/?id={}", link, hash.as_ref()),
            },
        }
    }
}
