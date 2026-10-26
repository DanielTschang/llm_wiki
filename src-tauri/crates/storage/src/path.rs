use std::fmt;

use crate::StoreError;

/// A Project-relative path: forward slashes, no leading slash, no `.` or `..`
/// segments, no empty segments. The empty path is the Project root.
///
/// Validation here replaces the `canonicalize` and symlink checks the
/// filesystem version needed; there are no symlinks to follow.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelPath(String);

impl RelPath {
    pub fn root() -> Self {
        Self(String::new())
    }

    pub fn parse(raw: &str) -> Result<Self, StoreError> {
        let invalid = |reason| StoreError::InvalidPath {
            path: raw.to_string(),
            reason,
        };
        if raw.is_empty() {
            return Err(invalid("empty"));
        }
        if raw.starts_with('/') {
            return Err(invalid("must be relative"));
        }
        if raw.contains('\\') {
            return Err(invalid("backslash"));
        }
        if raw.contains('\0') {
            return Err(invalid("NUL byte"));
        }
        for segment in raw.split('/') {
            match segment {
                "" => return Err(invalid("empty segment")),
                "." | ".." => return Err(invalid("dot segment")),
                _ => {}
            }
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').next().filter(|name| !name.is_empty())
    }

    pub fn extension(&self) -> Option<&str> {
        let name = self.file_name()?;
        let (stem, ext) = name.rsplit_once('.')?;
        (!stem.is_empty()).then_some(ext)
    }

    pub fn parent(&self) -> Option<RelPath> {
        if self.is_root() {
            return None;
        }
        Some(Self(
            self.0
                .rsplit_once('/')
                .map(|(parent, _)| parent.to_string())
                .unwrap_or_default(),
        ))
    }

    pub fn join(&self, name: &str) -> Result<RelPath, StoreError> {
        if self.is_root() {
            RelPath::parse(name)
        } else {
            RelPath::parse(&format!("{}/{name}", self.0))
        }
    }

    /// True when `self` is `dir` itself or lies beneath it. The root contains
    /// everything. Compares whole segments, so `wiki2/a` is not under `wiki`.
    pub fn is_within(&self, dir: &RelPath) -> bool {
        dir.is_root()
            || self.0 == dir.0
            || self
                .0
                .strip_prefix(dir.0.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    }

    /// Every directory above this path, nearest first, without the root.
    pub fn ancestors(&self) -> Vec<RelPath> {
        let mut out = Vec::new();
        let mut cursor = self.parent();
        while let Some(dir) = cursor {
            if dir.is_root() {
                break;
            }
            cursor = dir.parent();
            out.push(dir);
        }
        out
    }

    /// The first segment, e.g. `wiki` for `wiki/entities/a.md`.
    pub fn top_level(&self) -> &str {
        self.0.split('/').next().unwrap_or("")
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_paths() {
        for ok in [
            "a",
            "wiki/a.md",
            "raw/sources/案例/報告.pdf",
            ".llm-wiki/review.json",
        ] {
            assert_eq!(RelPath::parse(ok).unwrap().as_str(), ok);
        }
    }

    #[test]
    fn rejects_anything_that_could_escape_the_project() {
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../b",
            "a/./b",
            "a//b",
            "a/b/",
            "a\\b",
            "a\0b",
            ".",
        ] {
            assert!(RelPath::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn parent_walks_up_to_the_root() {
        let path = RelPath::parse("wiki/entities/a.md").unwrap();
        let dir = path.parent().unwrap();
        assert_eq!(dir.as_str(), "wiki/entities");
        let top = dir.parent().unwrap();
        assert_eq!(top.as_str(), "wiki");
        let root = top.parent().unwrap();
        assert!(root.is_root());
        assert_eq!(root.parent(), None);
    }

    #[test]
    fn within_compares_whole_segments() {
        let wiki = RelPath::parse("wiki").unwrap();
        assert!(RelPath::parse("wiki/a.md").unwrap().is_within(&wiki));
        assert!(wiki.is_within(&wiki));
        assert!(!RelPath::parse("wiki2/a.md").unwrap().is_within(&wiki));
        assert!(RelPath::parse("anything")
            .unwrap()
            .is_within(&RelPath::root()));
    }

    #[test]
    fn extension_ignores_dotfiles() {
        assert_eq!(RelPath::parse("a/b.md").unwrap().extension(), Some("md"));
        assert_eq!(RelPath::parse("a/.hidden").unwrap().extension(), None);
        assert_eq!(RelPath::parse("a/noext").unwrap().extension(), None);
    }

    #[test]
    fn ancestors_exclude_the_root_and_the_path_itself() {
        let path = RelPath::parse("wiki/entities/a.md").unwrap();
        let names: Vec<_> = path.ancestors().iter().map(|p| p.to_string()).collect();
        assert_eq!(names, ["wiki/entities", "wiki"]);
        assert!(RelPath::parse("a").unwrap().ancestors().is_empty());
    }

    #[test]
    fn join_validates_the_result() {
        let wiki = RelPath::parse("wiki").unwrap();
        assert_eq!(wiki.join("a.md").unwrap().as_str(), "wiki/a.md");
        assert!(wiki.join("../x").is_err());
        assert_eq!(RelPath::root().join("raw").unwrap().as_str(), "raw");
    }
}
