/// The struct representation of a `Commit`
#[derive(Debug, Clone)]
pub struct Commit {
    /// The 40 char hash
    pub hash: String,
    /// The commit subject
    pub subject: String,
    /// The component (if any)
    pub component: String,
    /// Any issues this commit closes
    pub closes: Vec<String>,
    /// Any issues this commit breaks
    pub breaks: Vec<String>,
    /// The commit type (or alias)
    pub commit_type: String,
}

/// A convienience type for multiple commits
pub type Commits = Vec<Commit>;
