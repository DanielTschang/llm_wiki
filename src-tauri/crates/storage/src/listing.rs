use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::store::{EntryKind, Meta};
use crate::{RelPath, StoreError};

pub(crate) struct FileInfo {
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub version: Option<u32>,
}

pub(crate) fn dir_meta(path: RelPath, modified: DateTime<Utc>) -> Meta {
    Meta {
        path,
        kind: EntryKind::Dir,
        size: 0,
        modified,
        version: None,
    }
}

/// Turns the flat set of file paths under `dir` into a listing: every file
/// when `recursive`, otherwise direct children with directories synthesised
/// from the paths beneath them. Shared by every store so they cannot disagree.
pub(crate) fn list_entries(
    dir: &RelPath,
    recursive: bool,
    files: impl IntoIterator<Item = (RelPath, FileInfo)>,
) -> Result<Vec<Meta>, StoreError> {
    let mut out: BTreeMap<RelPath, Meta> = BTreeMap::new();
    for (path, info) in files {
        if &path == dir || !path.is_within(dir) {
            continue;
        }
        let file = |path: RelPath| Meta {
            path,
            kind: EntryKind::File,
            size: info.size,
            modified: info.modified,
            version: info.version,
        };
        if recursive {
            out.insert(path.clone(), file(path));
            continue;
        }
        let rest = path.as_str()[dir.as_str().len()..].trim_start_matches('/');
        match rest.split_once('/') {
            None => {
                out.insert(path.clone(), file(path));
            }
            Some((child, _)) => {
                let child = dir.join(child)?;
                out.entry(child.clone())
                    .and_modify(|meta| meta.modified = meta.modified.max(info.modified))
                    .or_insert_with(|| dir_meta(child, info.modified));
            }
        }
    }
    Ok(out.into_values().collect())
}
