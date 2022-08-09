use std::{collections::BTreeMap, io};

use log::debug;
use time;

use crate::{clog::Clog, error::Result, fmt::FormatWriter, git::Commit, sectionmap::SectionMap};

/// Wraps a `std::io::Write` object to write `clog` output in a JSON format
///
/// # Example
///
/// ```no_run
/// # use std::fs::File;
/// # use clog::{SectionMap, Clog, fmt::JsonWriter};
/// let clog = Clog::new().unwrap();
///
/// // Get the commits we're interested in...
/// let sm = SectionMap::from_commits(clog.get_commits().unwrap());
///
/// // Create a file to hold our results, which the JsonWriter will wrap (note, .unwrap() is only
/// // used to keep the example short and concise)
/// let mut file = File::create("my_changelog.json").ok().unwrap();
///
/// // Create the JSON Writer
/// let mut writer = JsonWriter::new(&mut file);
///
/// // Use the JsonWriter to write the changelog
/// clog.write_changelog_with(&mut writer).unwrap();
/// ```
pub struct JsonWriter<'a>(&'a mut dyn io::Write);

impl<'a> JsonWriter<'a> {
    /// Creates a new instance of the `JsonWriter` struct using a
    /// `std::io::Write` object.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::io::{stdout, BufWriter};
    /// # use clog::{Clog, fmt::JsonWriter};
    /// let clog = Clog::new().unwrap();
    ///
    /// // Create a JsonWriter to wrap stdout
    /// let out = stdout();
    /// let mut out_buf = BufWriter::new(out.lock());
    /// let mut writer = JsonWriter::new(&mut out_buf);
    /// ```
    pub fn new<T: io::Write>(writer: &'a mut T) -> JsonWriter<'a> { JsonWriter(writer) }
}

impl<'a> JsonWriter<'a> {
    /// Writes the initial header inforamtion for a release
    fn write_header(&mut self, options: &Clog) -> Result<()> {
        write!(
            self.0,
            "\"header\":{{\"version\":{:?},\"patch_version\":{:?},\"subtitle\":{},",
            options.version,
            options.patch_ver,
            options.subtitle.as_deref().unwrap_or("null"),
        )?;

        let now = time::now_utc();
        let date = now.strftime("%Y-%m-%d")?;
        write!(self.0, "\"date\":\"{}\"}},", date).map_err(Into::into)
    }

    /// Writes a particular section of a changelog
    fn write_section(
        &mut self,
        options: &Clog,
        section: &BTreeMap<&String, &Vec<Commit>>,
    ) -> Result<()> {
        if section.is_empty() {
            write!(self.0, "\"commits\":null")?;
            return Ok(());
        }

        write!(self.0, "\"commits\":[")?;
        let mut s_it = section.iter().peekable();
        while let Some((component, entries)) = s_it.next() {
            let mut e_it = entries.iter().peekable();
            debug!("Writing component: {}", component);
            while let Some(entry) = e_it.next() {
                debug!("Writing commit: {}", &*entry.subject);
                write!(self.0, "{{\"component\":")?;
                if component.is_empty() {
                    write!(self.0, "null,")?;
                } else {
                    write!(self.0, "{:?},", component)?;
                }
                write!(
                    self.0,
                    "\"subject\":{:?},\"commit_link\":{:?},\"closes\":",
                    entry.subject,
                    options
                        .link_style
                        .commit_link(&*entry.hash, options.repo.as_deref())
                )?;

                if !entry.closes.is_empty() {
                    write!(self.0, "[")?;
                    let mut c_it = entry.closes.iter().peekable();
                    while let Some(issue) = c_it.next() {
                        write!(
                            self.0,
                            "{{\"issue\":{},\"issue_link\":{:?}}}",
                            issue,
                            options.link_style.issue_link(issue, options.repo.as_ref())
                        )?;
                        if c_it.peek().is_some() {
                            debug!("There are more close commits, adding comma");
                            write!(self.0, ",")?;
                        } else {
                            debug!("There are no more close commits, no comma required");
                        }
                    }
                    write!(self.0, "],")?;
                } else {
                    write!(self.0, "null,")?;
                }
                write!(self.0, "\"breaks\":")?;
                if !entry.breaks.is_empty() {
                    write!(self.0, "[")?;
                    let mut c_it = entry.closes.iter().peekable();
                    while let Some(issue) = c_it.next() {
                        write!(
                            self.0,
                            "{{\"issue\":{},\"issue_link\":{:?}}}",
                            issue,
                            options.link_style.issue_link(issue, options.repo.as_ref())
                        )?;
                        if c_it.peek().is_some() {
                            debug!("There are more breaks commits, adding comma");
                            write!(self.0, ",")?;
                        } else {
                            debug!("There are no more breaks commits, no comma required");
                        }
                    }
                    write!(self.0, "]}}")?;
                } else {
                    write!(self.0, "null}}")?;
                }
                if e_it.peek().is_some() {
                    debug!("There are more commits, adding comma");
                    write!(self.0, ",")?;
                } else {
                    debug!("There are no more commits, no comma required");
                }
            }
            if s_it.peek().is_some() {
                debug!("There are more sections, adding comma");
                write!(self.0, ",")?;
            } else {
                debug!("There are no more commits, no comma required");
            }
        }
        write!(self.0, "]").map_err(Into::into)
    }

    /// Writes some contents to the `Write` writer object
    #[allow(dead_code)]
    fn write(&mut self, content: &str) -> io::Result<()> { write!(self.0, "{}", content) }
}

impl<'a> FormatWriter for JsonWriter<'a> {
    fn write_changelog(&mut self, options: &Clog, sm: &SectionMap) -> Result<()> {
        debug!("Writing JSON changelog");
        write!(self.0, "{{")?;
        self.write_header(options)?;

        write!(self.0, "\"sections\":")?;
        let mut s_it = options
            .section_map
            .keys()
            .filter_map(|sec| sm.sections.get(sec).map(|compmap| (sec, compmap)))
            .peekable();
        if s_it.peek().is_some() {
            debug!("There are sections to write");
            write!(self.0, "[")?;
            while let Some((sec, compmap)) = s_it.next() {
                debug!("Writing section: {sec}");
                write!(self.0, "{{\"title\":{sec:?},")?;

                self.write_section(options, &compmap.iter().collect::<BTreeMap<_, _>>())?;

                write!(self.0, "}}")?;
                if s_it.peek().is_some() {
                    debug!("There are more sections, adding comma");
                    write!(self.0, ",")?;
                } else {
                    debug!("There are no more sections, no comma required");
                }
            }
            write!(self.0, "]")?;
        } else {
            debug!("There are no sections to write");
            write!(self.0, "null")?;
        }

        write!(self.0, "}}")?;
        debug!("Finished writing sections, flushing");
        self.0.flush().map_err(Into::into)
    }
}
