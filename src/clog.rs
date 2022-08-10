use std::{
    collections::HashMap,
    convert::AsRef,
    env,
    fs::File,
    io::{stdout, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    result::Result as StdResult,
};

use indexmap::IndexMap;
use log::debug;
use regex::Regex;

use crate::{
    config::RawCfg,
    error::{Error, Result},
    fmt::{ChangelogFormat, FormatWriter, JsonWriter, MarkdownWriter},
    git::{Commit, Commits},
    link_style::LinkStyle,
    sectionmap::SectionMap,
    DEFAULT_CONFIG_FILE,
};

fn regex_default() -> Regex { regex!(r"^([^:\(]+?)(?:\(([^\)]*?)?\))?:(.*)") }
fn closes_regex_default() -> Regex { regex!(r"(?:Closes|Fixes|Resolves)\s((?:#(\d+)(?:,\s)?)+)") }
fn breaks_regex_default() -> Regex { regex!(r"(?:Breaks|Broke)\s((?:#(\d+)(?:,\s)?)+)") }
fn breaking_regex_default() -> Regex { regex!(r"(?i:breaking)") }

/// The base struct used to set options and interact with the library.
#[derive(Debug, Clone)]
pub struct Clog {
    /// The repository used for the base of hyper-links
    pub repo: Option<String>,
    /// The link style to used for commit and issue hyper-links
    pub link_style: LinkStyle,
    /// The file to use as the old changelog data to be appended to anything new
    /// found.
    pub infile: Option<String>,
    /// The subtitle for the release
    pub subtitle: Option<String>,
    /// The file to use as the changelog output file (Defaults to `stdout`)
    pub outfile: Option<String>,
    /// Maps out the sections and aliases used to trigger those sections. The
    /// keys are the section name, and the values are an array of aliases.
    pub section_map: IndexMap<String, Vec<String>>,
    /// Maps out the components and aliases used to trigger those components.
    /// The keys are the component name, and the values are an array of
    /// aliases.
    pub component_map: HashMap<String, Vec<String>>,
    /// The git dir with all the meta-data (Typically the `.git` sub-directory
    /// of the project)
    pub git_dir: Option<PathBuf>,
    /// The format to output the changelog in (Defaults to Markdown)
    pub out_format: ChangelogFormat,
    /// The grep search pattern used to find commits we are interested in
    /// (Defaults to: "^ft|^feat|^fx|^fix|^perf|^unk|BREAKING\'")
    pub grep: String,
    /// The format of the commit output from `git log` (Defaults to:
    /// "%H%n%s%n%b%n==END==")
    pub format: String,
    /// The working directory of the git project (typically the project
    /// directory, or parent of the `.git` directory)
    pub git_work_tree: Option<PathBuf>,
    /// The regex used to get components, aliases, and messages
    pub regex: Regex,
    /// The regex used to get closes issue links
    pub closes_regex: Regex,
    /// The regex used to get closes issue links
    pub breaks_regex: Regex,
    pub breaking_regex: Regex,
    /// Where to start looking for commits using a hash (or short hash)
    pub from: Option<String>,
    /// Where to stop looking for commits using a hash (or short hash).
    /// (Defaults to `HEAD`)
    pub to: String,
    /// The version tag for the release (Defaults to the short hash of the
    /// latest commit)
    pub version: Option<String>,
    /// Whether or not this is a patch version update or not. Patch versions use
    /// a lower markdown header (`###` instead of `##` for major and minor
    /// releases)
    pub patch_ver: bool,
}

impl Default for Clog {
    fn default() -> Self {
        debug!("Creating default clog with Clog::default()");
        let mut sections = IndexMap::new();
        sections.insert(
            "Features".to_owned(),
            vec!["ft".to_owned(), "feat".to_owned()],
        );
        sections.insert(
            "Bug Fixes".to_owned(),
            vec!["fx".to_owned(), "fix".to_owned()],
        );
        sections.insert("Performance".to_owned(), vec!["perf".to_owned()]);
        sections.insert("Unknown".to_owned(), vec!["unk".to_owned()]);
        sections.insert("Breaking Changes".to_owned(), vec!["breaks".to_owned()]);

        Clog {
            grep: format!(
                "{}BREAKING'",
                sections
                    .values()
                    .map(|v| v
                        .iter()
                        .fold(String::new(), |acc, al| { acc + &format!("^{}|", al)[..] }))
                    .fold(String::new(), |acc, al| { acc + &format!("^{}|", al)[..] })
            ),
            format: "%H%n%s%n%b%n==END==".to_string(),
            repo: None,
            link_style: LinkStyle::Github,
            version: None,
            patch_ver: false,
            subtitle: None,
            from: None,
            to: "HEAD".to_string(),
            infile: None,
            outfile: None,
            section_map: sections,
            component_map: HashMap::new(),
            out_format: ChangelogFormat::Markdown,
            git_dir: None,
            git_work_tree: None,
            regex: regex_default(),
            closes_regex: closes_regex_default(),
            breaks_regex: breaks_regex_default(),
            breaking_regex: breaking_regex_default(),
        }
    }
}

impl TryFrom<RawCfg> for Clog {
    type Error = Error;

    fn try_from(cfg: RawCfg) -> StdResult<Self, Self::Error> {
        let mut clog = Self {
            repo: cfg.clog.repository,
            link_style: cfg.clog.link_style,
            subtitle: cfg.clog.subtitle,
            infile: cfg.clog.changelog.clone().or(cfg.clog.infile),
            outfile: cfg.clog.changelog.or(cfg.clog.outfile),
            section_map: cfg.sections,
            component_map: cfg.components,
            out_format: cfg.clog.output_format,
            git_dir: cfg.clog.git_dir,
            git_work_tree: cfg.clog.git_work_tree,
            ..Self::default()
        };
        if cfg.clog.from_latest_tag {
            clog.from = Some(clog.get_latest_tag()?);
        }
        Ok(clog)
    }
}

impl Clog {
    /// Creates a default `Clog` struct using the current working directory and
    /// searches for the default `.clog.toml` configuration file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// ```
    pub fn new() -> Result<Self> {
        debug!("Creating default clog with new()");
        debug!("Trying default config file");
        Clog::from_config(DEFAULT_CONFIG_FILE)
    }

    /// Creates a `Clog` struct using a specific git working directory OR
    /// project directory. Searches for the default configuration TOML file
    /// `.clog.toml`
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the
    /// working tree, and vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::with_git_work_tree("/myproject").unwrap();
    /// ```
    pub fn with_git_work_tree<P: AsRef<Path>>(dir: P) -> Result<Self> {
        debug!("Creating clog with \n\tdir: {:?}", dir.as_ref());
        Clog::_new(Some(dir.as_ref()), None)
    }

    /// Creates a `Clog` struct a custom named TOML configuration file. Sets the
    /// parent directory of the configuration file to the working tree and
    /// sibling `.git` directory as the git directory.
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the
    /// working tree, and vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::from_config("/myproject/clog_conf.toml").unwrap();
    /// ```
    pub fn from_config<P: AsRef<Path>>(cfg: P) -> Result<Self> {
        debug!("Creating clog with \n\tfile: {:?}", cfg.as_ref());
        Clog::_new(None, Some(cfg.as_ref()))
    }

    fn _new(dir: Option<&Path>, cfg: Option<&Path>) -> Result<Self> {
        debug!("Creating private clog with \n\tdir: {:?}", dir);
        // Determine if the cfg_file was relative or not
        let cfg = if let Some(cfg) = cfg {
            if cfg.is_relative() {
                debug!("file is relative");
                let cwd = match env::current_dir() {
                    Ok(d) => d,
                    Err(..) => return Err(Error::CurrentDir),
                };
                Path::new(&cwd).join(cfg)
            } else {
                debug!("file is absolute");
                cfg.to_path_buf()
            }
        } else {
            Path::new(DEFAULT_CONFIG_FILE).to_path_buf()
        };

        // if dir is None we assume whatever dir the cfg file is also contains the git
        // metadata
        let mut dir = dir.unwrap_or(&cfg).to_path_buf();
        dir.pop();
        let git_dir;
        let git_work_tree;
        if dir.ends_with(".git") {
            debug!("dir ends with .git");
            let mut wd = dir.clone();
            git_dir = Some(wd.clone());
            wd.pop();
            git_work_tree = Some(wd);
        } else {
            debug!("dir doesn't end with .git");
            let mut gd = dir.clone();
            git_work_tree = Some(gd.clone());
            gd.push(".git");
            git_dir = Some(gd);
        }
        Ok(Clog {
            git_dir,
            git_work_tree,
            ..Clog::try_config_file(&cfg)?
        })
    }

    // Try and create a clog object from a config file
    fn try_config_file(cfg_file: &Path) -> Result<Self> {
        debug!("Trying to use config file: {:?}", cfg_file);
        let mut toml_f = File::open(cfg_file)?;
        let mut toml_s = String::with_capacity(100);

        toml_f.read_to_string(&mut toml_s)?;

        let cfg: RawCfg = toml::from_str(&toml_s[..])?;
        cfg.try_into()
    }

    /// Sets the grep search pattern for finding commits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().grep("BREAKS");
    /// ```
    #[must_use]
    pub fn grep<S: Into<String>>(mut self, g: S) -> Clog {
        self.grep = g.into();
        self
    }

    /// Sets the format for `git log` output
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().format("%H%n%n==END==");
    /// ```
    #[must_use]
    pub fn format<S: Into<String>>(mut self, f: S) -> Clog {
        self.format = f.into();
        self
    }

    /// Sets the repository used for the base of hyper-links
    ///
    /// **NOTE:** Leave off the trailing `.git`
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new()
    ///     .unwrap()
    ///     .repository("https://github.com/thoughtram/clog");
    /// ```
    #[must_use]
    pub fn repository<S: Into<String>>(mut self, r: S) -> Clog {
        self.repo = Some(r.into());
        self
    }

    /// Sets the link style to use for hyper-links
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::{Clog, LinkStyle};
    /// let clog = Clog::new().unwrap().link_style(LinkStyle::Stash);
    /// ```
    #[must_use]
    pub fn link_style(mut self, l: LinkStyle) -> Clog {
        self.link_style = l;
        self
    }

    /// Sets the version for the release
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().version("v0.2.1-beta3");
    /// ```
    #[must_use]
    pub fn version<S: Into<String>>(mut self, v: S) -> Clog {
        self.version = Some(v.into());
        self
    }

    /// Sets the subtitle for the release
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().subtitle("My Awesome Release Title");
    /// ```
    #[must_use]
    pub fn subtitle<S: Into<String>>(mut self, s: S) -> Clog {
        self.subtitle = Some(s.into());
        self
    }

    /// Sets how far back to begin searching commits using a short hash or full
    /// hash
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().from("6d8183f");
    /// ```
    #[must_use]
    pub fn from<S: Into<String>>(mut self, f: S) -> Clog {
        self.from = Some(f.into());
        self
    }

    /// Sets what point to stop searching for commits using a short hash or full
    /// hash (Defaults to `HEAD`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().to("123abc4d");
    /// ```
    #[must_use]
    pub fn to<S: Into<String>>(mut self, t: S) -> Clog {
        self.to = t.into();
        self
    }

    /// Sets the changelog file to output or prepend to (Defaults to `stdout` if
    /// omitted)
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().changelog("/myproject/my_changelog.md");
    /// ```
    #[must_use]
    pub fn changelog<S: Into<String> + Clone>(mut self, c: S) -> Clog {
        self.infile = Some(c.clone().into());
        self.outfile = Some(c.into());
        self
    }

    /// Sets the changelog output file to output or prepend to (Defaults to
    /// `stdout` if omitted), this is useful inconjunction with
    /// `Clog::infile()` because it allows to read previous commits from one
    /// place and output to another.
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// **NOTE:** This should *not* be used in conjunction with
    /// `Clog::changelog()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().outfile("/myproject/my_changelog.md");
    /// ```
    #[must_use]
    pub fn outfile<S: Into<String>>(mut self, c: S) -> Clog {
        self.outfile = Some(c.into());
        self
    }

    /// Sets the changelog input file to read previous commits or changelog data
    /// from. This is useful inconjunction with `Clog::infile()` because it
    /// allows to read previous commits from one place and output to
    /// another.
    ///
    /// **NOTE:** Anything set here will override anything in a configuration
    /// TOML file
    ///
    /// **NOTE:** This should *not* be used in conjunction with
    /// `Clog::changelog()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new()
    ///     .unwrap()
    ///     .infile("/myproject/my_old_changelog.md");
    /// ```
    #[must_use]
    pub fn infile<S: Into<String>>(mut self, c: S) -> Clog {
        self.infile = Some(c.into());
        self
    }

    /// Sets the `git` metadata directory (typically `.git` child of your
    /// project working tree)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().git_dir("/myproject/.git");
    /// ```
    #[must_use]
    pub fn git_dir<P: AsRef<Path>>(mut self, d: P) -> Clog {
        self.git_dir = Some(d.as_ref().to_path_buf());
        self
    }

    /// Sets the `git` working tree directory (typically your project directory)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().git_work_tree("/myproject");
    /// ```
    #[must_use]
    pub fn git_work_tree<P: AsRef<Path>>(mut self, d: P) -> Clog {
        self.git_work_tree = Some(d.as_ref().to_path_buf());
        self
    }

    /// Sets whether or not this is a patch release (defaults to `false`)
    ///
    /// **NOTE:** Setting this to true will cause the release subtitle to use a
    /// smaller markdown heading
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap().patch_ver(true);
    /// ```
    #[must_use]
    pub fn patch_ver(mut self, p: bool) -> Clog {
        self.patch_ver = p;
        self
    }

    /// The format of output for the changelog (Defaults to Markdown)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::{fmt::ChangelogFormat,Clog};
    /// let clog = Clog::new().unwrap().output_format(ChangelogFormat::Json);
    /// ```
    #[must_use]
    pub fn output_format(mut self, f: ChangelogFormat) -> Clog {
        self.out_format = f;
        self
    }

    /// Retrieves a `Vec<Commit>` of only commits we care about.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let commits = clog.get_commits();
    /// ```
    pub fn get_commits(&self) -> Result<Commits> {
        let range = if let Some(from) = self.from.as_ref() {
            format!("{from}..{}", self.to)
        } else {
            "HEAD".to_owned()
        };

        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("log")
            .arg("-E")
            .arg(&format!("--grep={}", self.grep))
            .arg(&format!("--format={}", self.format))
            .arg(&range)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .split("\n==END==\n")
            .filter_map(|commit_str| self.parse_raw_commit(commit_str).ok())
            .filter(|entry| entry.commit_type != "Unknown")
            .collect())
    }

    #[doc(hidden)]
    pub fn parse_raw_commit(&self, commit_str: &str) -> Result<Commit> {
        let mut lines = commit_str.lines();
        let hash = lines.next().unwrap_or_default();

        let (subject, component, commit_type) =
            match lines.next().and_then(|s| self.regex.captures(s)) {
                Some(caps) => {
                    let section = caps.get(1).map(|c| c.as_str()).unwrap_or_default();
                    let commit_type = self
                        .section_for(section)
                        .ok_or(Error::UnknownComponent(section.into()))?;
                    let component = caps.get(2).map(|component| {
                        let component = component.as_str();
                        match self.component_for(component) {
                            Some(alias) => alias.clone(),
                            None => component.to_owned(),
                        }
                    });
                    let subject = caps.get(3).map(|c| c.as_str());
                    (subject, component, commit_type)
                }
                None => (
                    None,
                    None,
                    self.section_for("unk")
                        .ok_or(Error::UnknownComponent("unk".into()))?,
                ),
            };
        let mut closes = vec![];
        let mut breaks = vec![];
        for line in lines {
            if let Some(caps) = self.closes_regex.captures(line) {
                if let Some(cap) = caps.get(2) {
                    closes.push(cap.as_str().to_owned());
                }
            }
            if let Some(caps) = self.breaks_regex.captures(line) {
                if let Some(cap) = caps.get(2) {
                    breaks.push(cap.as_str().to_owned());
                }
            } else if self.breaking_regex.captures(line).is_some() {
                breaks.push(String::new());
            }
        }

        Ok(Commit {
            hash: hash.to_string(),
            subject: subject.unwrap().to_owned(),
            component: component.unwrap_or_default(),
            closes,
            breaks,
            commit_type: commit_type.to_string(),
        })
    }

    /// Retrieves the latest tag from the git directory
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let tag = clog.get_latest_tag().unwrap();
    /// ```
    pub fn get_latest_tag(&self) -> Result<String> {
        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("rev-list")
            .arg("--tags")
            .arg("--max-count=1")
            .output()?;
        let buf = String::from_utf8_lossy(&output.stdout);

        Ok(buf.trim_matches('\n').to_owned())
    }

    /// Retrieves the latest tag version from the git directory
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let tag_ver = clog.get_latest_tag_ver();
    /// ```
    pub fn get_latest_tag_ver(&self) -> String {
        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("describe")
            .arg("--tags")
            .arg("--abbrev=0")
            .output()
            .unwrap_or_else(|e| panic!("Failed to run 'git describe' with error: {}", e));

        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Retrieves the hash of the most recent commit from the git directory
    /// (i.e. HEAD)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let head_hash = clog.get_last_commit();
    /// ```
    pub fn get_last_commit(&self) -> String {
        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .unwrap_or_else(|e| panic!("Failed to run 'git rev-parse' with error: {}", e));

        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn get_git_work_tree(&self) -> String {
        // Check if user supplied a local git dir and working tree
        if self.git_work_tree.is_none() && self.git_dir.is_none() {
            // None was provided
            "".to_owned()
        } else if self.git_dir.is_some() {
            // user supplied both
            format!(
                "--work-tree={}",
                self.git_work_tree.clone().unwrap().to_str().unwrap()
            )
        } else {
            // user only supplied a working tree i.e. /home/user/mycode
            let mut w = self.git_work_tree.clone().unwrap();
            w.pop();
            format!("--work-tree={}", w.to_str().unwrap())
        }
    }

    fn get_git_dir(&self) -> String {
        // Check if user supplied a local git dir and working tree
        if self.git_dir.is_none() && self.git_work_tree.is_none() {
            // None was provided
            "".to_owned()
        } else if self.git_work_tree.is_some() {
            // user supplied both
            format!(
                "--git-dir={}",
                self.git_dir.clone().unwrap().to_str().unwrap()
            )
        } else {
            // user only supplied a git dir i.e. /home/user/mycode/.git
            let mut g = self.git_dir.clone().unwrap();
            g.push(".git");
            format!("--git-dir={}", g.to_str().unwrap())
        }
    }

    /// Retrieves the section title for a given alias
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let section = clog.section_for("feat");
    /// assert_eq!(Some("Features"), section);
    /// ```
    pub fn section_for(&self, alias: &str) -> Option<&str> {
        self.section_map
            .iter()
            .find(|&(_, v)| v.iter().any(|s| s == alias))
            .map(|(k, _)| &**k)
    }

    /// Retrieves the full component name for a given alias (if one is defined)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// let component = clog.component_for("will_be_none");
    /// assert_eq!(None, component);
    /// ```
    pub fn component_for(&self, alias: &str) -> Option<&String> {
        self.component_map
            .iter()
            .filter(|&(_, v)| v.iter().any(|c| c == alias))
            .map(|(k, _)| k)
            .next()
    }

    /// Writes the changelog using whatever options have been specified thus
    /// far.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    /// clog.write_changelog();
    /// ```
    pub fn write_changelog(&self) -> Result<()> {
        debug!("Writing changelog with preset options");
        if let Some(ref cl) = self.outfile {
            debug!("outfile set to: {:?}", cl);
            self.write_changelog_to(cl)
        } else if let Some(ref cl) = self.infile {
            debug!("outfile not set but infile set to: {:?}", cl);
            self.write_changelog_from(cl)
        } else {
            debug!("outfile and infile not set using stdout");
            let out = stdout();
            let mut out_buf = BufWriter::new(out.lock());
            match self.out_format {
                ChangelogFormat::Markdown => {
                    let mut writer = MarkdownWriter::new(&mut out_buf);
                    self.write_changelog_with(&mut writer)
                }
                ChangelogFormat::Json => {
                    let mut writer = JsonWriter::new(&mut out_buf);
                    self.write_changelog_with(&mut writer)
                }
            }
        }
    }

    /// Writes the changelog to a specified file, and prepends new commits if
    /// file exists, or creates the file if it doesn't.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    ///
    /// clog.write_changelog_to("/myproject/new_changelog.md")
    ///     .unwrap();
    /// ```
    pub fn write_changelog_to<P: AsRef<Path>>(&self, cl: P) -> Result<()> {
        debug!("Writing changelog to file: {:?}", cl.as_ref());
        let mut contents = String::with_capacity(256);
        if let Some(ref infile) = self.infile {
            debug!("infile set to: {:?}", infile);
            File::open(infile)
                .map(|mut f| f.read_to_string(&mut contents).ok())
                .ok();
        } else {
            debug!("infile not set, trying the outfile");
            File::open(cl.as_ref())
                .map(|mut f| f.read_to_string(&mut contents).ok())
                .ok();
        }
        contents.shrink_to_fit();

        let mut file = File::create(cl.as_ref())?;
        match self.out_format {
            ChangelogFormat::Markdown => {
                let mut writer = MarkdownWriter::new(&mut file);
                self.write_changelog_with(&mut writer)?;
            }
            ChangelogFormat::Json => {
                let mut writer = JsonWriter::new(&mut file);
                self.write_changelog_with(&mut writer)?;
            }
        }
        write!(&mut file, "\n\n\n")?;

        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    /// Writes the changelog from a specified input file, and appends new
    /// commits
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap();
    ///
    /// clog.write_changelog_from("/myproject/new_old_changelog.md")
    ///     .unwrap();
    /// ```
    pub fn write_changelog_from<P: AsRef<Path>>(&self, cl: P) -> Result<()> {
        debug!("Writing changelog from file: {:?}", cl.as_ref());
        let mut contents = String::with_capacity(256);
        File::open(cl.as_ref())
            .map(|mut f| f.read_to_string(&mut contents).ok())
            .ok();
        contents.shrink_to_fit();

        if let Some(ref ofile) = self.outfile {
            debug!("outfile set to: {:?}", ofile);
            let mut file = File::create(ofile)?;
            match self.out_format {
                ChangelogFormat::Markdown => {
                    let mut writer = MarkdownWriter::new(&mut file);
                    self.write_changelog_with(&mut writer)?;
                }
                ChangelogFormat::Json => {
                    let mut writer = JsonWriter::new(&mut file);
                    self.write_changelog_with(&mut writer)?;
                }
            }
            file.write_all(contents.as_bytes())?;
        } else {
            debug!("outfile not set, using stdout");
            let out = stdout();
            let mut out_buf = BufWriter::new(out.lock());
            {
                match self.out_format {
                    ChangelogFormat::Markdown => {
                        let mut writer = MarkdownWriter::new(&mut out_buf);
                        self.write_changelog_with(&mut writer)?;
                    }
                    ChangelogFormat::Json => {
                        let mut writer = JsonWriter::new(&mut out_buf);
                        self.write_changelog_with(&mut writer)?;
                    }
                }
            }
            write!(&mut out_buf, "\n\n\n")?;

            out_buf.write_all(contents.as_bytes())?;
        }

        Ok(())
    }

    /// Writes a changelog with a specified `FormatWriter` format
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clog::{Clog, fmt::{FormatWriter, MarkdownWriter}};
    /// # use std::io;
    /// let clog = Clog::new().unwrap();
    ///
    /// // Write changelog to stdout in Markdown format
    /// let out = io::stdout();
    /// let mut out_buf = io::BufWriter::new(out.lock());
    /// let mut writer = MarkdownWriter::new(&mut out_buf);
    ///
    /// clog.write_changelog_with(&mut writer).unwrap();
    /// ```
    pub fn write_changelog_with<W>(&self, writer: &mut W) -> Result<()>
    where
        W: FormatWriter,
    {
        debug!("Writing changelog from writer");
        let sm = SectionMap::from_commits(self.get_commits()?);

        writer.write_changelog(self, &sm)
    }
}
