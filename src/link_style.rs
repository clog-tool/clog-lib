/// Determines the hyperlink style used in commit and issue links. Defaults to `LinksStyle::Github`
///
/// # Example
///
/// ```no_run
/// # use clog::{LinkStyle, Clog};
/// let mut clog = Clog::new().unwrap();
/// clog.link_style(LinkStyle::Stash);
/// ```
clog_enum!{
    #[derive(Debug)]
    pub enum LinkStyle {
        Github,
        Gitlab,
        Stash,
        Cgit,
        Gitweb
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
            "" => format!("{}", issue.as_ref()),
            link => {
                match *self {
                    LinkStyle::Github => format!("{}/issues/{}", link, issue.as_ref()),
                    LinkStyle::Gitlab => format!("{}/issues/{}", link, issue.as_ref()),
                    LinkStyle::Stash => format!("{}", issue.as_ref()),
                    // cgit does not support issues
                    LinkStyle::Cgit => format!("{}", issue.as_ref()),
                    // gitweb does not support issues
                    LinkStyle::Gitweb => format!("{}", issue.as_ref()),
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
    ///
    /// # Example
    /// Note that for `LinkStyle::Gitweb` the actual repository name has to be given as part of the parameter string of the URL:
    ///
    /// ```no_run
    /// # use clog::{LinkStyle, Clog};
    /// let link = LinkStyle::Gitweb;
    /// let commit = link.commit_link("deadbeef", "http://example.com/gitweb/?p=foo.git");
    ///
    /// assert_eq!("http://example.com/gitweb/?p=foo.git;a=commit;h=deadbeef", commit);
    /// ```
    pub fn commit_link<S: AsRef<str>>(&self, hash: S, repo: S) -> String {
        match repo.as_ref() {
            "" => format!("{}", &hash.as_ref()[0..8]),
            link => {
                match *self {
                    LinkStyle::Github => format!("{}/commit/{}", link, hash.as_ref()),
                    LinkStyle::Gitlab => format!("{}/commit/{}", link, hash.as_ref()),
                    LinkStyle::Stash => format!("{}/commits/{}", link, hash.as_ref()),
                    LinkStyle::Cgit => format!("{}/commit/?id={}", link, hash.as_ref()),
                    LinkStyle::Gitweb => format!("{};a=commit;h={}", link, hash.as_ref()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitweb_commit_link() {
        let link = LinkStyle::Gitweb;
        let hash = "deadbeef";
        let commit = link.commit_link(hash, "http://example.com/gitweb/?p=foo.git");
        assert_eq!(format!("http://example.com/gitweb/?p=foo.git;a=commit;h={}", &hash), commit);
    }

    #[test]
    fn test_gitweb_issue_link() {
       let link = LinkStyle::Gitweb;
       let issue = link.issue_link("42", "http://example.com/gitweb/?p=foo.git");
       assert_eq!("42", issue);
    }
}
