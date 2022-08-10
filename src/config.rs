use std::{collections::HashMap, path::PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::{fmt::ChangelogFormat, link_style::LinkStyle};

#[derive(Debug, Clone, Deserialize)]
pub struct RawCfg {
    pub clog: RawClogCfg,
    #[serde(default)]
    pub sections: IndexMap<String, Vec<String>>,
    #[serde(default)]
    pub components: HashMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RawClogCfg {
    pub changelog: Option<String>,
    pub from_latest_tag: bool,
    pub repository: Option<String>,
    pub infile: Option<String>,
    pub subtitle: Option<String>,
    pub outfile: Option<String>,
    pub git_dir: Option<PathBuf>,
    pub git_work_tree: Option<PathBuf>,
    pub link_style: LinkStyle,
    pub output_format: ChangelogFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config() {
        let cfg = include_str!("../examples/clog.toml");
        let res = toml::from_str(cfg);
        assert!(res.is_ok(), "{res:?}");
        let cfg: RawCfg = res.unwrap();

        assert_eq!(
            cfg.clog.repository,
            Some("https://github.com/clog-tool/clog-lib".into())
        );
        assert_eq!(cfg.clog.subtitle, Some("my awesome title".into()));
        assert_eq!(cfg.clog.link_style, LinkStyle::Github);
        assert_eq!(cfg.clog.changelog, Some("mychangelog.md".into()));
        assert_eq!(cfg.clog.outfile, Some("MyChangelog.md".into()));
        assert_eq!(cfg.clog.infile, Some("My_old_changelog.md".into()));
        assert_eq!(cfg.clog.output_format, ChangelogFormat::Json);
        assert_eq!(cfg.clog.git_work_tree, Some("/myproject".into()));
        assert_eq!(cfg.clog.git_dir, Some("/myproject/.git".into()));
        assert!(cfg.clog.from_latest_tag);
        assert_eq!(
            cfg.sections.get("MySection"),
            Some(&vec!["mysec".into(), "ms".into()])
        );
        assert_eq!(
            cfg.sections.get("Another Section"),
            Some(&vec!["another".into()])
        );
        assert_eq!(
            cfg.components.get("MyLongComponentName"),
            Some(&vec!["long".into(), "comp".into()])
        );
    }

    #[test]
    fn dogfood_config() {
        let cfg = include_str!("../.clog.toml");
        let res = toml::from_str(cfg);
        assert!(res.is_ok(), "{res:?}");
        let cfg: RawCfg = res.unwrap();

        assert_eq!(
            cfg.clog.repository,
            Some("https://github.com/clog-tool/clog-lib".into())
        );
        assert_eq!(cfg.clog.link_style, LinkStyle::Github);
        assert!(cfg.clog.from_latest_tag);
    }
}
