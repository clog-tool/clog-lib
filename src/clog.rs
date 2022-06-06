use std::collections::HashMap;
use std::convert::AsRef;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{stdout, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use indexmap::IndexMap;
use regex::Regex;
use toml::{Parser, Value};

use error::Error;
use fmt::{ChangelogFormat, FormatWriter, JsonWriter, MarkdownWriter, WriterResult};
use git::{Commit, Commits};
use link_style::LinkStyle;
use sectionmap::SectionMap;

use CLOG_CONFIG_FILE;

/// Convienience type for returning results of building a `Clog` struct
///
/// # Example
///
/// ```no_run
/// # use clog::Clog;
/// let clog = Clog::new().unwrap_or_else(|e| {
///     // Prints the BuilderResult error and exits appropriately
///     e.exit();
/// });
/// ```
pub type BuilderResult = Result<Clog, Error>;

/// The base struct used to set options and interact with the library.
pub struct Clog {
    /// The grep search pattern used to find commits we are interested in (Defaults to:
    /// "^ft|^feat|^fx|^fix|^perf|^unk|BREAKING\'")
    pub grep: String,
    /// The format of the commit output from `git log` (Defaults to: "%H%n%s%n%b%n==END==")
    pub format: String,
    /// The repository used for the base of hyper-links
    pub repo: String,
    /// The link style to used for commit and issue hyper-links
    pub link_style: LinkStyle,
    /// The version tag for the release (Defaults to the short hash of the latest commit)
    pub version: String,
    /// Whether or not this is a patch version update or not. Patch versions use a lower markdown
    /// header (`###` instead of `##` for major and minor releases)
    pub patch_ver: bool,
    /// The subtitle for the release
    pub subtitle: String,
    /// Where to start looking for commits using a hash (or short hash)
    pub from: String,
    /// Where to stop looking for commits using a hash (or short hash). (Defaults to `HEAD`)
    pub to: String,
    /// The file to use as the old changelog data to be appended to anything new found.
    pub infile: Option<String>,
    /// The file to use as the changelog output file (Defaults to `stdout`)
    pub outfile: Option<String>,
    /// Maps out the sections and aliases used to trigger those sections. The keys are the section
    /// name, and the values are an array of aliases.
    pub section_map: IndexMap<String, Vec<String>>,
    /// Maps out the components and aliases used to trigger those components. The keys are the
    /// component name, and the values are an array of aliases.
    pub component_map: HashMap<String, Vec<String>>,
    /// The git dir with all the meta-data (Typically the `.git` sub-directory of the project)
    pub git_dir: Option<PathBuf>,
    /// The working directory of the git project (typically the project directory, or parent of the
    /// `.git` directory)
    pub git_work_tree: Option<PathBuf>,
    /// The regex used to get components, aliases, and messages
    pub regex: Regex,
    /// The regex used to get closes issue links
    pub closes_regex: Regex,
    /// The regex used to get closes issue links
    pub breaks_regex: Regex,
    pub breaking_regex: Regex,
    /// The format to output the changelog in (Defaults to Markdown)
    pub out_format: ChangelogFormat,
}

impl fmt::Debug for Clog {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{
            grep: {:?}
            format: {:?}
            repo: {:?}
            link_style: {:?}
            version: {:?}
            patch_ver: {:?}
            subtitle: {:?}
            from: {:?}
            to: {:?}
            infile: {:?}
            outfile: {:?}
            section_map: {:?}
            component_map: {:?}
            git_dir: {:?}
            git_work_tree: {:?}
            regex: {:?}
            closes_regex: {:?}
            breaks_regex: {:?}
            breaking_regex: {:?}
            out_format: {:?}
        }}",
            self.grep,
            self.format,
            self.repo,
            self.link_style,
            self.version,
            self.patch_ver,
            self.subtitle,
            self.from,
            self.to,
            self.infile,
            self.outfile,
            self.section_map,
            self.component_map,
            self.git_dir,
            self.git_work_tree,
            self.regex,
            self.closes_regex,
            self.breaks_regex,
            self.breaking_regex,
            self.out_format,
        )
    }
}

impl Clog {
    fn _new() -> Clog {
        debugln!("Creating default clog with _new()");
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
            format: "%H%n%s%n%b%n==END==".to_owned(),
            repo: "".to_owned(),
            link_style: LinkStyle::Github,
            version: "".to_owned(),
            patch_ver: false,
            subtitle: "".to_owned(),
            from: "".to_owned(),
            to: "HEAD".to_owned(),
            infile: None,
            outfile: None,
            section_map: sections,
            component_map: HashMap::new(),
            out_format: ChangelogFormat::Markdown,
            git_dir: None,
            git_work_tree: None,
            regex: regex!(r"^([^:\(]+?)(?:\(([^\)]*?)?\))?:(.*)"),
            closes_regex: regex!(r"(?:Closes|Fixes|Resolves)\s((?:#(\d+)(?:,\s)?)+)"),
            breaks_regex: regex!(r"(?:Breaks|Broke)\s((?:#(\d+)(?:,\s)?)+)"),
            breaking_regex: regex!(r"(?i:breaking)"),
        }
    }

    /// Creates a default `Clog` struct using the current working directory and searches for the
    /// default `.clog.toml` configuration file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::new().unwrap_or_else(|e| {
    ///     // Prints the error to stderr and exits
    ///     e.exit();
    /// });
    /// ```
    pub fn new() -> BuilderResult {
        debugln!("Creating default clog with new()");
        debugln!("Trying default config file");
        Clog::from_file(CLOG_CONFIG_FILE)
    }

    /// Creates a `Clog` struct using a specific git working directory and project directory as
    /// well as a custom named TOML configuration file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::with_all("/myproject/.git",
    ///                           "/myproject",
    ///                           "/myproject/clog_conf.toml").unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    /// ```
    pub fn with_all<P: AsRef<Path>>(git_dir: P, work_tree: P, cfg_file: P) -> BuilderResult {
        debugln!(
            "Creating clog with \n\tgit_dir: {:?}\n\twork_tree: {:?}\n\tcfg_file: {:?}",
            git_dir.as_ref(),
            work_tree.as_ref(),
            cfg_file.as_ref()
        );
        Clog::with_dirs(git_dir, work_tree)?.try_config_file(cfg_file.as_ref())
    }

    /// Creates a `Clog` struct using a specific git working directory OR project directory as
    /// well as a custom named TOML configuration file.
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the working tree, and
    /// vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::with_dir_and_file("/myproject",
    ///                           "/myproject/clog_conf.toml").unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    /// ```
    pub fn with_dir_and_file<P: AsRef<Path>>(dir: P, cfg_file: P) -> BuilderResult {
        debugln!(
            "Creating clog with \n\tdir: {:?}\n\tcfg_file: {:?}",
            dir.as_ref(),
            cfg_file.as_ref()
        );
        Clog::_with_dir(dir)?.try_config_file(cfg_file.as_ref())
    }

    fn _with_dir<P: AsRef<Path>>(dir: P) -> BuilderResult {
        debugln!("Creating private clog with \n\tdir: {:?}", dir.as_ref());
        let mut clog = Clog::_new();
        if dir.as_ref().ends_with(".git") {
            debugln!("dir ends with .git");
            let mut wd = dir.as_ref().to_path_buf();
            clog.git_dir = Some(wd.clone());
            wd.pop();
            clog.git_work_tree = Some(wd);
        } else {
            debugln!("dir doesn't end with .git");
            let mut gd = dir.as_ref().to_path_buf();
            clog.git_work_tree = Some(gd.clone());
            gd.push(".git");
            clog.git_dir = Some(gd);
        }

        debugln!("Returning clog:\n{:?}", clog);
        Ok(clog)
    }

    /// Creates a `Clog` struct using a specific git working directory OR project directory.
    /// Searches for the default configuration TOML file `.clog.toml`
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the working tree, and
    /// vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::with_dir("/myproject").unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    /// ```
    pub fn with_dir<P: AsRef<Path>>(dir: P) -> BuilderResult {
        debugln!("Creating clog with \n\tdir: {:?}", dir.as_ref());
        Clog::_with_dir(dir)?.try_config_file(Path::new(CLOG_CONFIG_FILE))
    }

    /// Creates a `Clog` struct using a specific git working directory AND a project directory.
    /// Searches for the default configuration TOML file `.clog.toml`
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the working tree, and
    /// vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::with_dirs("/myproject", "/myproject/.git").unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    /// ```
    pub fn with_dirs<P: AsRef<Path>>(git_dir: P, work_tree: P) -> BuilderResult {
        debugln!(
            "Creating clog with \n\tgit_dir: {:?}\n\twork_tree: {:?}",
            git_dir.as_ref(),
            work_tree.as_ref()
        );
        let mut clog = Clog::_new();
        clog.git_dir = Some(git_dir.as_ref().to_path_buf());
        clog.git_work_tree = Some(work_tree.as_ref().to_path_buf());
        clog.try_config_file(Path::new(CLOG_CONFIG_FILE))
    }

    /// Creates a `Clog` struct a custom named TOML configuration file. Sets the parent directory
    /// of the configuration file to the working tree and sibling `.git` directory as the git
    /// directory.
    ///
    /// **NOTE:** If you specify a `.git` folder the parent will be used as the working tree, and
    /// vice versa.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let clog = Clog::from_file("/myproject/clog_conf.toml").unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    /// ```
    pub fn from_file<P: AsRef<Path>>(file: P) -> BuilderResult {
        debugln!("Creating clog with \n\tfile: {:?}", file.as_ref());
        // Determine if the cfg_file was relative or not
        let cfg_file = if file.as_ref().is_relative() {
            debugln!("file is relative");
            let cwd = match env::current_dir() {
                Ok(d) => d,
                Err(..) => return Err(Error::CurrentDirErr),
            };
            Path::new(&cwd).join(file.as_ref())
        } else {
            debugln!("file is absolute");
            file.as_ref().to_path_buf()
        };

        // We assume whatever dir the .clog.toml file is also contains the git metadata
        let mut dir = cfg_file.clone();
        dir.pop();
        Clog::with_dir_and_file(dir, cfg_file)
    }

    // Try and create a clog object from a config file
    fn try_config_file(mut self, cfg_file: &Path) -> BuilderResult {
        debugln!("Trying to use config file: {:?}", cfg_file);
        let mut toml_from_latest = None;
        let mut toml_repo = None;
        let mut toml_subtitle = None;
        let mut toml_link_style = None;
        let mut toml_outfile = None;
        let mut toml_infile = None;
        let mut toml_changelog = None;
        let mut toml_format = None;

        if let Ok(ref mut toml_f) = File::open(cfg_file) {
            debugln!("Found file");
            let mut toml_s = String::with_capacity(100);

            if let Err(..) = toml_f.read_to_string(&mut toml_s) {
                return Err(Error::TomlReadErr);
            }

            toml_s.shrink_to_fit();

            let mut toml = Parser::new(&toml_s[..]);

            let toml_table = match toml.parse() {
                Some(table) => table,
                None => {
                    return Err(Error::ConfigParseErr);
                }
            };

            let clog_table = match toml_table.get("clog") {
                Some(table) => table,
                None => {
                    return Err(Error::ConfigFormatErr);
                }
            };

            toml_from_latest = clog_table
                .lookup("from-latest-tag")
                .unwrap_or(&Value::Boolean(false))
                .as_bool();
            toml_repo = match clog_table.lookup("repository") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => Some("".to_owned()),
            };
            toml_subtitle = match clog_table.lookup("subtitle") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => Some("".to_owned()),
            };
            toml_link_style = match clog_table.lookup("link-style") {
                Some(val) => match val.as_str().unwrap_or("github").parse::<LinkStyle>() {
                    Ok(style) => Some(style),
                    Err(..) => {
                        return Err(Error::LinkStyleErr);
                    }
                },
                None => Some(LinkStyle::Github),
            };
            toml_outfile = match clog_table.lookup("outfile") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => None,
            };
            toml_infile = match clog_table.lookup("infile") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => None,
            };
            toml_changelog = match clog_table.lookup("changelog") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => None,
            };
            toml_format = match clog_table.lookup("output-format") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None => None,
            };
            match toml_table.get("sections") {
                Some(table) => match table.as_table() {
                    Some(table) => {
                        for (sec, val) in table.iter() {
                            if let Some(vec) = val.as_slice() {
                                let alias_vec = vec
                                    .iter()
                                    .map(|v| v.as_str().unwrap_or("").to_owned())
                                    .collect::<Vec<_>>();
                                self.section_map.insert(sec.to_owned(), alias_vec);
                            }
                        }
                    }
                    None => (),
                },
                None => (),
            };
            match toml_table.get("components") {
                Some(table) => match table.as_table() {
                    Some(table) => {
                        for (comp, val) in table.iter() {
                            if let Some(vec) = val.as_slice() {
                                let alias_vec = vec
                                    .iter()
                                    .map(|v| v.as_str().unwrap_or("").to_owned())
                                    .collect::<Vec<_>>();
                                self.component_map.insert(comp.to_owned(), alias_vec);
                            }
                        }
                    }
                    None => (),
                },
                None => (),
            };
        } else {
            debugln!("File didn't exist");
        };

        if toml_from_latest.unwrap_or(false) {
            self.from = self.get_latest_tag();
        }

        if let Some(repo) = toml_repo {
            self.repo = repo;
        }

        if let Some(ls) = toml_link_style {
            self.link_style = ls;
        }

        if let Some(subtitle) = toml_subtitle {
            self.subtitle = subtitle;
        }

        if let Some(outfile) = toml_outfile {
            self.outfile = Some(outfile);
        }

        if let Some(infile) = toml_infile {
            self.infile = Some(infile);
        }

        if let Some(format) = toml_format {
            match format.parse::<ChangelogFormat>() {
                Ok(val) => self.out_format = val,
                Err(..) => return Err(Error::ConfigFormatErr),
            }
        }

        if let Some(ref cl) = toml_changelog {
            self.outfile = Some(cl.to_owned());
            self.infile = Some(cl.to_owned());
        }

        debugln!("Returning clog:\n{:?}", self);
        Ok(self)
    }

    /// Sets the grep search pattern for finding commits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.grep("BREAKS");
    /// ```
    pub fn grep<S: Into<String>>(&mut self, g: S) -> &mut Clog {
        self.grep = g.into();
        self
    }

    /// Sets the format for `git log` output
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.format("%H%n%n==END==");
    /// ```
    pub fn format<S: Into<String>>(&mut self, f: S) -> &mut Clog {
        self.format = f.into();
        self
    }

    /// Sets the repository used for the base of hyper-links
    ///
    /// **NOTE:** Leave off the trailing `.git`
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.repository("https://github.com/thoughtram/clog");
    /// ```
    pub fn repository<S: Into<String>>(&mut self, r: S) -> &mut Clog {
        self.repo = r.into();
        self
    }

    /// Sets the link style to use for hyper-links
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::{Clog, LinkStyle};
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.link_style(LinkStyle::Stash);
    /// ```
    pub fn link_style(&mut self, l: LinkStyle) -> &mut Clog {
        self.link_style = l;
        self
    }

    /// Sets the version for the release
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.version("v0.2.1-beta3");
    /// ```
    pub fn version<S: Into<String>>(&mut self, v: S) -> &mut Clog {
        self.version = v.into();
        self
    }

    /// Sets the subtitle for the release
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.subtitle("My Awesome Release Title");
    /// ```
    pub fn subtitle<S: Into<String>>(&mut self, s: S) -> &mut Clog {
        self.subtitle = s.into();
        self
    }

    /// Sets how far back to begin searching commits using a short hash or full hash
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.from("6d8183f");
    /// ```
    pub fn from<S: Into<String>>(&mut self, f: S) -> &mut Clog {
        self.from = f.into();
        self
    }

    /// Sets what point to stop searching for commits using a short hash or full hash (Defaults to
    /// `HEAD`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.to("123abc4d");
    /// ```
    pub fn to<S: Into<String>>(&mut self, t: S) -> &mut Clog {
        self.to = t.into();
        self
    }

    /// Sets the changelog file to output or prepend to (Defaults to `stdout` if omitted)
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.changelog("/myproject/my_changelog.md");
    /// ```
    pub fn changelog<S: Into<String> + Clone>(&mut self, c: S) -> &mut Clog {
        self.infile = Some(c.clone().into());
        self.outfile = Some(c.into());
        self
    }

    /// Sets the changelog output file to output or prepend to (Defaults to `stdout` if omitted),
    /// this is useful inconjunction with `Clog::infile()` because it allows to read previous
    /// commits from one place and output to another.
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// **NOTE:** This should *not* be used in conjunction with `Clog::changelog()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.outfile("/myproject/my_changelog.md");
    /// ```
    pub fn outfile<S: Into<String>>(&mut self, c: S) -> &mut Clog {
        self.outfile = Some(c.into());
        self
    }

    /// Sets the changelog input file to read previous commits or changelog data from. This is
    /// useful inconjunction with `Clog::infile()` because it allows to read previous commits from
    /// one place and output to another.
    ///
    /// **NOTE:** Anything set here will override anything in a configuration TOML file
    ///
    /// **NOTE:** This should *not* be used in conjunction with `Clog::changelog()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.infile("/myproject/my_old_changelog.md");
    /// ```
    pub fn infile<S: Into<String>>(&mut self, c: S) -> &mut Clog {
        self.infile = Some(c.into());
        self
    }

    /// Sets the `git` metadata directory (typically `.git` child of your project working tree)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.git_dir("/myproject/.git");
    /// ```
    pub fn git_dir<P: AsRef<Path>>(&mut self, d: P) -> &mut Clog {
        self.git_dir = Some(d.as_ref().to_path_buf());
        self
    }

    /// Sets the `git` working tree directory (typically your project directory)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.git_work_tree("/myproject");
    /// ```
    pub fn git_work_tree<P: AsRef<Path>>(&mut self, d: P) -> &mut Clog {
        self.git_work_tree = Some(d.as_ref().to_path_buf());
        self
    }

    /// Sets whether or not this is a patch release (defaults to `false`)
    ///
    /// **NOTE:** Setting this to true will cause the release subtitle to use a smaller markdown
    /// heading
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.patch_ver(true);
    /// ```
    pub fn patch_ver(&mut self, p: bool) -> &mut Clog {
        self.patch_ver = p;
        self
    }

    /// The format of output for the changelog (Defaults to Markdown)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// # use clog::fmt::ChangelogFormat;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.output_format(ChangelogFormat::Json);
    /// ```
    pub fn output_format(&mut self, f: ChangelogFormat) -> &mut Clog {
        self.out_format = f;
        self
    }

    /// Retrieves a `Vec<Commit>` of only commits we care about.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// let commits = clog.get_commits();
    /// ```
    pub fn get_commits(&self) -> Commits {
        let range = match &self.from[..] {
            "" => "HEAD".to_owned(),
            _ => format!("{}..{}", self.from, self.to),
        };

        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("log")
            .arg("-E")
            .arg(&format!("--grep={}", self.grep))
            .arg(&format!("--format={}", self.format))
            .arg(&range)
            .output()
            .unwrap_or_else(|e| panic!("Failed to run 'git log' with error: {}", e));

        String::from_utf8_lossy(&output.stdout)
            .split("\n==END==\n")
            .map(|commit_str| self.parse_raw_commit(commit_str))
            .filter(|entry| entry.commit_type != "Unknown")
            .collect()
    }

    #[doc(hidden)]
    pub fn parse_raw_commit(&self, commit_str: &str) -> Commit {
        let mut lines = commit_str.lines();

        let hash = lines.next().unwrap_or("").to_owned();

        let (subject, component, commit_type) =
            match lines.next().and_then(|s| self.regex.captures(s)) {
                Some(caps) => {
                    let commit_type = self.section_for(caps.at(1).unwrap_or("")).to_owned();
                    let component = caps.at(2).map(|component| {
                        match self.component_for(component) {
                            Some(alias) => alias.clone(),
                            None => component.to_owned(),
                        }
                        .to_owned()
                    });
                    let subject = caps.at(3);
                    (subject, component, commit_type)
                }
                None => (
                    Some(""),
                    Some("".to_owned()),
                    self.section_for("unk").clone(),
                ),
            };
        let mut closes = vec![];
        let mut breaks = vec![];
        for line in lines {
            if let Some(caps) = self.closes_regex.captures(line) {
                if let Some(cap) = caps.at(2) {
                    closes.push(cap.to_owned());
                }
            }
            if let Some(caps) = self.breaks_regex.captures(line) {
                if let Some(cap) = caps.at(2) {
                    breaks.push(cap.to_owned());
                }
            } else if self.breaking_regex.captures(line).is_some() {
                breaks.push("".to_owned());
            }
        }

        Commit {
            hash: hash,
            subject: subject.unwrap().to_owned(),
            component: component.unwrap_or("".to_string()).to_owned(),
            closes: closes,
            breaks: breaks,
            commit_type: commit_type,
        }
    }

    /// Retrieves the latest tag from the git directory
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// let tag = clog.get_latest_tag();
    /// ```
    pub fn get_latest_tag(&self) -> String {
        let output = Command::new("git")
            .arg(&self.get_git_dir()[..])
            .arg(&self.get_git_work_tree()[..])
            .arg("rev-list")
            .arg("--tags")
            .arg("--max-count=1")
            .output()
            .unwrap_or_else(|e| panic!("Failed to run 'git rev-list' with error: {}", e));
        let buf = String::from_utf8_lossy(&output.stdout);

        buf.trim_matches('\n').to_owned()
    }

    /// Retrieves the latest tag version from the git directory
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
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

    /// Retrieves the hash of the most recent commit from the git directory (i.e. HEAD)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
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
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// let section = clog.section_for("feat");
    /// assert_eq!("Features", section);
    /// ```
    pub fn section_for(&self, alias: &str) -> &String {
        self.section_map
            .iter()
            .filter(|&(_, v)| v.iter().any(|s| s == alias))
            .map(|(k, _)| k)
            .next()
            .unwrap_or(
                self.section_map
                    .keys()
                    .filter(|&k| k == "Unknown")
                    .next()
                    .unwrap(),
            )
    }

    /// Retrieves the full component name for a given alias (if one is defined)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
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

    /// Writes the changelog using whatever options have been specified thus far.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// clog.write_changelog();
    /// ```
    pub fn write_changelog(&self) -> WriterResult {
        debugln!("Writing changelog with preset options");
        if let Some(ref cl) = self.outfile {
            debugln!("outfile set to: {:?}", cl);
            self.write_changelog_to(cl)
        } else if let Some(ref cl) = self.infile {
            debugln!("outfile not set but infile set to: {:?}", cl);
            self.write_changelog_from(cl)
        } else {
            debugln!("outfile and infile not set using stdout");
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

    /// Writes the changelog to a specified file, and prepends new commits if file exists, or
    /// creates the file if it doesn't.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| e.exit());
    ///
    /// clog.write_changelog_to("/myproject/new_changelog.md").unwrap_or_else(|e| {
    ///     // Prints the error and exits appropriately
    ///     e.exit();
    /// });
    /// ```
    pub fn write_changelog_to<P: AsRef<Path>>(&self, cl: P) -> WriterResult {
        debugln!("Writing changelog to file: {:?}", cl.as_ref());
        let mut contents = String::with_capacity(256);
        if let Some(ref infile) = self.infile {
            debugln!("infile set to: {:?}", infile);
            File::open(infile)
                .map(|mut f| f.read_to_string(&mut contents).ok())
                .ok();
        } else {
            debugln!("infile not set, trying the outfile");
            File::open(cl.as_ref())
                .map(|mut f| f.read_to_string(&mut contents).ok())
                .ok();
        }
        contents.shrink_to_fit();

        if let Ok(mut file) = File::create(cl.as_ref()) {
            {
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
            }
            if let Err(..) = write!(&mut file, "\n\n\n") {
                return Err(Error::WriteErr);
            }

            if let Err(..) = file.write(contents.as_bytes()) {
                return Err(Error::WriteErr);
            }
        } else {
            return Err(Error::CreateFileErr);
        }

        Ok(())
    }

    /// Writes the changelog from a specified input file, and appends new commits
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// let mut clog = Clog::new().unwrap_or_else(|e| e.exit());
    ///
    /// clog.write_changelog_from("/myproject/new_old_changelog.md").unwrap_or_else(|e| {
    ///     // Prints the error and exits appropriately
    ///     e.exit();
    /// });
    /// ```
    pub fn write_changelog_from<P: AsRef<Path>>(&self, cl: P) -> WriterResult {
        debugln!("Writing changelog from file: {:?}", cl.as_ref());
        let mut contents = String::with_capacity(256);
        File::open(cl.as_ref())
            .map(|mut f| f.read_to_string(&mut contents).ok())
            .ok();
        contents.shrink_to_fit();

        if let Some(ref ofile) = self.outfile {
            debugln!("outfile set to: {:?}", ofile);
            if let Ok(mut file) = File::create(ofile) {
                {
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
                }

                if let Err(..) = file.write(contents.as_bytes()) {
                    return Err(Error::WriteErr);
                }
            } else {
                return Err(Error::CreateFileErr);
            }
        } else {
            debugln!("outfile not set, using stdout");
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
            if let Err(..) = write!(&mut out_buf, "\n\n\n") {
                return Err(Error::WriteErr);
            }

            if let Err(..) = out_buf.write(contents.as_bytes()) {
                return Err(Error::WriteErr);
            }
        }

        Ok(())
    }

    /// Writes a changelog with a specified `FormatWriter` format
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use clog::Clog;
    /// # use clog::fmt::{FormatWriter, MarkdownWriter};
    /// # use std::io;
    /// let clog = Clog::new().unwrap_or_else(|e| {
    ///     e.exit();
    /// });
    ///
    /// // Write changelog to stdout in Markdown format
    /// let out = io::stdout();
    /// let mut out_buf = io::BufWriter::new(out.lock());
    /// let mut writer = MarkdownWriter::new(&mut out_buf);
    ///
    /// clog.write_changelog_with(&mut writer).unwrap_or_else(|e| {
    ///     // Prints the error and exits appropriately
    ///     e.exit();
    /// });
    /// ```
    pub fn write_changelog_with<W>(&self, writer: &mut W) -> WriterResult
    where
        W: FormatWriter,
    {
        debugln!("Writing changelog from writer");
        let sm = SectionMap::from_commits(self.get_commits());

        writer.write_changelog(self, &sm)?;

        Ok(())
    }
}
