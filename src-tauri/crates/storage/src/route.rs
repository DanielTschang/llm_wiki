use crate::RelPath;

/// Where a path's bytes live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Text documents: the `pages` table.
    Pages,
    /// Binary files: MinIO objects described by the `blobs` table.
    Blobs,
    /// `.llm-wiki/**`: folder-era state files, served from their tables.
    State,
}

/// Text under `wiki/` plus the root `purpose.md` and `schema.md` are pages.
/// `.llm-wiki/` is state. Everything else, including `raw/`, `wiki/media/`
/// and `agent-workspace/`, is a blob.
pub fn route(path: &RelPath) -> Backend {
    const ROOT_DOCUMENTS: [&str; 2] = ["purpose.md", "schema.md"];
    const TEXT_EXTENSIONS: [&str; 3] = ["md", "markdown", "txt"];

    match path.top_level() {
        ".llm-wiki" => Backend::State,
        "wiki" if is_text(path, &TEXT_EXTENSIONS) && !is_media(path) => Backend::Pages,
        _ if ROOT_DOCUMENTS.contains(&path.as_str()) => Backend::Pages,
        _ => Backend::Blobs,
    }
}

/// The `blobs.kind` for a path that [`route`] sent to [`Backend::Blobs`].
pub fn blob_kind(path: &RelPath) -> &'static str {
    let in_cache = path.as_str().split('/').any(|segment| segment == ".cache");
    if in_cache || path.is_within(&RelPath::parse("raw/parsed").expect("literal is a valid path")) {
        "extracted"
    } else if path.top_level() == "agent-workspace" {
        "workspace"
    } else if path.top_level() == "raw" {
        "source"
    } else {
        "asset"
    }
}

fn is_text(path: &RelPath, extensions: &[&str]) -> bool {
    path.extension()
        .is_some_and(|ext| extensions.iter().any(|t| ext.eq_ignore_ascii_case(t)))
}

fn is_media(path: &RelPath) -> bool {
    path.is_within(&RelPath::parse("wiki/media").expect("literal is a valid path"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend(path: &str) -> Backend {
        route(&RelPath::parse(path).unwrap())
    }

    #[test]
    fn wiki_text_and_root_documents_are_pages() {
        for path in [
            "wiki/index.md",
            "wiki/entities/a.md",
            "wiki/log.MD",
            "purpose.md",
            "schema.md",
        ] {
            assert_eq!(backend(path), Backend::Pages, "{path}");
        }
    }

    #[test]
    fn binaries_and_raw_files_are_blobs() {
        for path in [
            "raw/sources/report.pdf",
            "raw/sources/notes.md",
            "raw/parsed/report.md",
            "wiki/media/fig1.png",
            "wiki/media/notes.md",
            "wiki/chart.png",
            "agent-workspace/deck/index.html",
            "other/purpose.md",
        ] {
            assert_eq!(backend(path), Backend::Blobs, "{path}");
        }
    }

    #[test]
    fn blob_kinds_follow_the_folder_layout() {
        let kind = |p: &str| blob_kind(&RelPath::parse(p).unwrap());
        assert_eq!(kind("raw/sources/a/report.pdf"), "source");
        assert_eq!(kind("raw/sources/a/.cache/report.pdf"), "extracted");
        assert_eq!(kind("raw/parsed/a/report.pdf.md"), "extracted");
        assert_eq!(kind("wiki/media/fig/1.png"), "asset");
        assert_eq!(kind("agent-workspace/deck/index.html"), "workspace");
    }

    #[test]
    fn llm_wiki_directory_is_state() {
        for path in [
            ".llm-wiki/review.json",
            ".llm-wiki/chats/c1.json",
            ".llm-wiki/lancedb",
        ] {
            assert_eq!(backend(path), Backend::State, "{path}");
        }
    }
}
