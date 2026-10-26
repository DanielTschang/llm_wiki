//! The file commands of [`super::fs`] for platform mode: the same operations,
//! served from the storage layer instead of the host disk. Paths arrive as
//! `/projects/<uuid>/<relative path>` and are resolved by the platform's
//! [`ProjectLocator`](llm_wiki_storage::ProjectLocator); anything else is
//! refused. Directories are implicit, so `create_directory` has nothing to do
//! and a directory with no files lists as empty.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use llm_wiki_storage::{
    route, Backend, Body, EntryKind, Precondition, ProjectId, RelPath, StoreError, WriteCtx,
};

use super::fs::{
    cache_format_path_for, cache_path_for, entry_is_visible, extract_office_text, extract_pdf_text,
    org_to_markdown, uses_anydoc_cache, FileBase64, EBOOK_EXTS, IMAGE_EXTS, LEGACY_DOC_EXTS,
    MEDIA_EXTS, OFFICE_CACHE_FORMAT, OFFICE_EXTS,
};
use crate::panic_guard::run_guarded;
use crate::platform::Platform;
use crate::types::wiki::FileNode;

type Res<T> = Result<T, String>;

fn store_error(error: StoreError) -> String {
    error.to_string()
}

fn resolve(platform: &Platform, path: &str) -> Res<(ProjectId, RelPath)> {
    platform.locator.resolve(path).map_err(store_error)
}

/// The path callers see for `rel` inside `project`.
fn virtual_path(platform: &Platform, project: ProjectId, rel: &RelPath) -> String {
    let root = platform.locator.project_path(project);
    if rel.is_root() {
        root
    } else {
        format!("{root}/{rel}")
    }
}

fn ctx(tool: &str) -> WriteCtx {
    WriteCtx::new(tool)
}

fn extension_of(rel: &RelPath) -> String {
    rel.extension().unwrap_or("").to_ascii_lowercase()
}

/// A file's content as text; `None` when it is not UTF-8.
async fn read_text(
    platform: &Platform,
    project: ProjectId,
    rel: &RelPath,
) -> Result<String, StoreError> {
    let (body, _) = platform.files.read(project, rel).await?;
    match body {
        Body::Text(text) => Ok(text),
        Body::Bytes(bytes) => String::from_utf8(bytes).map_err(|_| StoreError::InvalidPath {
            path: rel.to_string(),
            reason: "not valid UTF-8 text",
        }),
    }
}

async fn put(
    platform: &Platform,
    project: ProjectId,
    rel: &RelPath,
    body: Body,
    tool: &str,
) -> Res<()> {
    platform
        .files
        .write(project, rel, body, Precondition::Any, &ctx(tool))
        .await
        .map(|_| ())
        .map_err(|error| format!("Failed to write file '{rel}': {error}"))
}

/// Bytes headed for a page must be text; anything else is stored as given.
fn body_for(rel: &RelPath, bytes: Vec<u8>) -> Res<Body> {
    if route(rel) != Backend::Pages {
        return Ok(Body::Bytes(bytes));
    }
    String::from_utf8(bytes)
        .map(Body::Text)
        .map_err(|_| format!("'{rel}' is a page and must be text"))
}

pub async fn write_file(platform: &Platform, path: &str, contents: String) -> Res<()> {
    let (project, rel) = resolve(platform, path)?;
    put(
        platform,
        project,
        &rel,
        Body::Text(contents),
        "ui.write_file",
    )
    .await
}

pub async fn write_file_base64(platform: &Platform, path: &str, base64: String) -> Res<()> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let (project, rel) = resolve(platform, path)?;
    let bytes = B64
        .decode(base64.as_bytes())
        .map_err(|error| format!("Invalid base64 for '{path}': {error}"))?;
    put(
        platform,
        project,
        &rel,
        body_for(&rel, bytes)?,
        "ui.write_file_base64",
    )
    .await
}

/// Every store write is already atomic, so this is `write_file`.
pub async fn write_file_atomic(platform: &Platform, path: &str, contents: String) -> Res<()> {
    let (project, rel) = resolve(platform, path)?;
    put(
        platform,
        project,
        &rel,
        Body::Text(contents),
        "ui.write_file_atomic",
    )
    .await
}

pub async fn create_directory(platform: &Platform, path: &str) -> Res<()> {
    resolve(platform, path).map(|_| ())
}

pub async fn delete_file(platform: &Platform, path: &str) -> Res<()> {
    let (project, rel) = resolve(platform, path)?;
    platform
        .files
        .delete(project, &rel, &ctx("ui.delete_file"))
        .await
        .map(|_| ())
        .map_err(|error| format!("Failed to delete '{path}': {error}"))
}

pub async fn copy_file(platform: &Platform, source: &str, destination: &str) -> Res<()> {
    let (from_project, from) = resolve(platform, source)?;
    let (to_project, to) = resolve(platform, destination)?;
    let (body, _) = platform
        .files
        .read(from_project, &from)
        .await
        .map_err(|error| format!("Failed to copy '{source}' to '{destination}': {error}"))?;
    put(platform, to_project, &to, body, "ui.copy_file").await
}

#[derive(Default)]
struct Dir {
    subdirs: BTreeMap<String, Dir>,
    files: BTreeSet<String>,
}

impl Dir {
    fn insert(&mut self, segments: &[&str]) {
        match segments {
            [] => {}
            [file] => {
                self.files.insert((*file).to_string());
            }
            [dir, rest @ ..] => self
                .subdirs
                .entry((*dir).to_string())
                .or_default()
                .insert(rest),
        }
    }

    /// Directories first, then files, each in name order, down to `max_depth`
    /// levels.
    fn nodes(&self, base: &str, level: usize, max_depth: usize) -> Vec<FileNode> {
        if level >= max_depth {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (name, sub) in &self.subdirs {
            let path = format!("{base}/{name}");
            let children = sub.nodes(&path, level + 1, max_depth);
            out.push(FileNode {
                name: name.clone(),
                path,
                is_dir: true,
                children: (!children.is_empty()).then_some(children),
            });
        }
        for name in &self.files {
            out.push(FileNode {
                name: name.clone(),
                path: format!("{base}/{name}"),
                is_dir: false,
                children: None,
            });
        }
        out
    }
}

/// The files under `dir`, as segments relative to it, with hidden entries
/// left out unless asked for.
async fn visible_files(
    platform: &Platform,
    project: ProjectId,
    dir: &RelPath,
    include_hidden: bool,
) -> Res<Vec<RelPath>> {
    let listed = platform
        .files
        .list(project, dir, true)
        .await
        .map_err(store_error)?;
    Ok(listed
        .into_iter()
        .filter(|meta| meta.kind == EntryKind::File)
        .map(|meta| meta.path)
        .filter(|path| {
            let rest = path.as_str()[dir.as_str().len()..].trim_start_matches('/');
            rest.split('/')
                .all(|segment| entry_is_visible(segment, include_hidden))
        })
        .collect())
}

pub async fn list_directory(
    platform: &Platform,
    path: &str,
    include_hidden: Option<bool>,
    max_depth: Option<usize>,
) -> Res<Vec<FileNode>> {
    let include_hidden = include_hidden.unwrap_or(false);
    let max_depth = max_depth.unwrap_or(30).clamp(1, 30);
    let (project, dir) = resolve(platform, path)?;
    let held = platform
        .files
        .stat(project, &dir)
        .await
        .map_err(store_error)?;
    if held.is_some_and(|meta| meta.kind == EntryKind::File) {
        return Err(format!("Path is not a directory: '{path}'"));
    }
    let mut tree = Dir::default();
    for file in visible_files(platform, project, &dir, include_hidden).await? {
        let rest = file.as_str()[dir.as_str().len()..].trim_start_matches('/');
        tree.insert(&rest.split('/').collect::<Vec<_>>());
    }
    Ok(tree.nodes(&virtual_path(platform, project, &dir), 0, max_depth))
}

/// Copies every visible file under `source`; returns the destination paths.
pub async fn copy_directory(
    platform: &Platform,
    source: &str,
    destination: &str,
) -> Res<Vec<String>> {
    let (from_project, from) = resolve(platform, source)?;
    let (to_project, to) = resolve(platform, destination)?;
    let held = platform
        .files
        .stat(from_project, &from)
        .await
        .map_err(store_error)?;
    if !held.is_some_and(|meta| meta.kind == EntryKind::Dir) {
        return Err(format!("'{source}' is not a directory"));
    }
    let mut copied = Vec::new();
    for file in visible_files(platform, from_project, &from, false).await? {
        let suffix = &file.as_str()[from.as_str().len()..];
        let target = RelPath::parse(&format!("{to}{suffix}")).map_err(store_error)?;
        let (body, _) = platform
            .files
            .read(from_project, &file)
            .await
            .map_err(|error| format!("Failed to copy '{file}': {error}"))?;
        put(platform, to_project, &target, body, "ui.copy_directory").await?;
        copied.push(virtual_path(platform, to_project, &target));
    }
    Ok(copied)
}

pub async fn file_exists(platform: &Platform, path: &str) -> Res<bool> {
    let (project, rel) = resolve(platform, path)?;
    let held = platform
        .files
        .stat(project, &rel)
        .await
        .map_err(store_error)?;
    Ok(held.is_some())
}

async fn stat_of(platform: &Platform, path: &str) -> Res<llm_wiki_storage::Meta> {
    let (project, rel) = resolve(platform, path)?;
    platform
        .files
        .stat(project, &rel)
        .await
        .map_err(store_error)?
        .ok_or_else(|| format!("Failed to get metadata for '{path}': not found"))
}

pub async fn get_file_modified_time(platform: &Platform, path: &str) -> Res<u64> {
    let meta = stat_of(platform, path).await?;
    Ok(meta.modified.timestamp_millis().max(0) as u64)
}

pub async fn get_file_size(platform: &Platform, path: &str) -> Res<u64> {
    Ok(stat_of(platform, path).await?.size)
}

pub async fn get_file_md5(platform: &Platform, path: &str) -> Res<String> {
    use md5::{Digest, Md5};
    let (project, rel) = resolve(platform, path)?;
    let (body, _) = platform
        .files
        .read(project, &rel)
        .await
        .map_err(|error| format!("Failed to open file '{path}': {error}"))?;
    Ok(format!("{:x}", Md5::digest(body.into_bytes())))
}

pub async fn read_file_as_base64(platform: &Platform, path: &str) -> Res<FileBase64> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let (project, rel) = resolve(platform, path)?;
    let (body, _) = platform
        .files
        .read(project, &rel)
        .await
        .map_err(|error| format!("Failed to read '{path}': {error}"))?;
    Ok(FileBase64 {
        base64: B64.encode(body.into_bytes()),
        mime_type: super::fs::mime_for_extension(&extension_of(&rel)).to_string(),
    })
}

/// Extensions whose text is extracted by a parser that needs a real file.
fn needs_parser(ext: &str) -> bool {
    ext == "pdf" || OFFICE_EXTS.contains(&ext) || EBOOK_EXTS.contains(&ext)
}

/// Extensions `preprocess_file` caches the extracted text of.
fn is_cacheable(ext: &str) -> bool {
    needs_parser(ext) || ext == "org"
}

fn cache_rel(rel: &RelPath) -> Res<RelPath> {
    let cache = cache_path_for(Path::new(rel.as_str()));
    RelPath::parse(&cache.to_string_lossy().replace('\\', "/")).map_err(store_error)
}

/// The extracted text saved by `preprocess_file`, if it is at least as new
/// as the file it came from.
pub(crate) async fn read_cache(platform: &Platform, project: ProjectId, rel: &RelPath) -> Option<String> {
    let cache = cache_rel(rel).ok()?;
    if uses_anydoc_cache(Path::new(rel.as_str())) {
        let marker = RelPath::parse(
            &cache_format_path_for(Path::new(rel.as_str()))
                .to_string_lossy()
                .replace('\\', "/"),
        )
        .ok()?;
        if read_text(platform, project, &marker).await.ok()?.trim() != OFFICE_CACHE_FORMAT {
            return None;
        }
    }
    let original = platform.files.stat(project, rel).await.ok()??;
    let cached = platform.files.stat(project, &cache).await.ok()??;
    if cached.modified < original.modified {
        return None;
    }
    read_text(platform, project, &cache).await.ok()
}

async fn write_cache(
    platform: &Platform,
    project: ProjectId,
    rel: &RelPath,
    text: &str,
) -> Res<()> {
    let cache = cache_rel(rel)?;
    let anydoc = uses_anydoc_cache(Path::new(rel.as_str()));
    let marker = RelPath::parse(
        &cache_format_path_for(Path::new(rel.as_str()))
            .to_string_lossy()
            .replace('\\', "/"),
    )
    .map_err(store_error)?;
    if anydoc {
        // Drop the validity marker before replacing the text, so a failure in
        // between costs a rebuild and never lets stale text be accepted.
        let _ = platform
            .files
            .delete(project, &marker, &ctx("preprocess"))
            .await;
    }
    put(
        platform,
        project,
        &cache,
        Body::Text(text.to_string()),
        "preprocess",
    )
    .await?;
    if anydoc {
        put(
            platform,
            project,
            &marker,
            Body::Text(OFFICE_CACHE_FORMAT.to_string()),
            "preprocess",
        )
        .await?;
    }
    Ok(())
}

/// A scratch folder, removed when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs a parser that wants a local file. The file is copied into a scratch
/// folder at its Project-relative path, so the PDF parser still finds the
/// `raw/sources` layout it uses to place extracted images; those images are
/// then uploaded under `wiki/media/<name>` and the links in the text are
/// rewritten from the scratch path to the Project path. The scratch folder is
/// the only disk this touches, and it is gone afterwards.
async fn extract_with_parser(
    platform: &Platform,
    project: ProjectId,
    rel: &RelPath,
    ext: &str,
    include_images: bool,
) -> Res<String> {
    let (body, _) = platform
        .files
        .read(project, rel)
        .await
        .map_err(|error| format!("Failed to read '{rel}': {error}"))?;
    let scratch =
        Scratch(std::env::temp_dir().join(format!("llm-wiki-extract-{}", uuid::Uuid::new_v4())));
    let local = scratch.0.join(rel.as_str());
    let bytes = body.into_bytes();
    let ext_owned = ext.to_string();
    let local_for_parser = local.clone();
    let text = tokio::task::spawn_blocking(move || {
        run_guarded("extract_with_parser", || {
            if let Some(parent) = local_for_parser.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create scratch folder: {e}"))?;
            }
            std::fs::write(&local_for_parser, bytes)
                .map_err(|e| format!("Failed to write scratch file: {e}"))?;
            let path = local_for_parser.to_string_lossy().to_string();
            match ext_owned.as_str() {
                "pdf" => extract_pdf_text(&path, include_images),
                e if EBOOK_EXTS.contains(&e) => {
                    crate::commands::ebook::extract_ebook_text(&path, e)
                }
                e => extract_office_text(&path, e),
            }
        })
    })
    .await
    .map_err(|e| format!("extraction blocking task join error: {e}"))??;

    if ext != "pdf" || !include_images {
        return Ok(text);
    }
    let stem = Path::new(rel.file_name().unwrap_or(""))
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("")
        .to_string();
    let media_local = scratch.0.join("wiki").join("media").join(&stem);
    if stem.is_empty() || !media_local.is_dir() {
        return Ok(text);
    }
    let media_rel = RelPath::parse(&format!("wiki/media/{stem}")).map_err(store_error)?;
    for entry in walkdir::WalkDir::new(&media_local)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let inside = entry
            .path()
            .strip_prefix(&media_local)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let target = RelPath::parse(&format!("{media_rel}/{inside}")).map_err(store_error)?;
        let bytes = tokio::fs::read(entry.path())
            .await
            .map_err(|e| format!("Failed to read extracted image: {e}"))?;
        put(
            platform,
            project,
            &target,
            Body::Bytes(bytes),
            "extract_images",
        )
        .await?;
    }
    let scratch_prefix = media_local.to_string_lossy().replace('\\', "/");
    Ok(text.replace(
        &scratch_prefix,
        &virtual_path(platform, project, &media_rel),
    ))
}

pub async fn read_file(
    platform: &Platform,
    path: &str,
    extract_images: Option<bool>,
) -> Res<String> {
    let (project, rel) = resolve(platform, path)?;
    let ext = extension_of(&rel);
    let name = rel.file_name().unwrap_or("").to_string();

    if is_cacheable(&ext) {
        if let Some(cached) = read_cache(platform, project, &rel).await {
            return Ok(cached);
        }
    }

    let size = || async {
        platform
            .files
            .stat(project, &rel)
            .await
            .ok()
            .flatten()
            .map_or(0, |meta| meta.size)
    };
    match ext.as_str() {
        "org" => read_text(platform, project, &rel)
            .await
            .map(|content| org_to_markdown(&content))
            .map_err(|error| read_error(path, error)),
        e if needs_parser(e) => {
            extract_with_parser(platform, project, &rel, e, extract_images.unwrap_or(true)).await
        }
        e if IMAGE_EXTS.contains(&e) => Ok(format!(
            "[Image: {name} ({:.1} KB)]",
            size().await as f64 / 1024.0
        )),
        e if MEDIA_EXTS.contains(&e) => Ok(format!(
            "[Media: {name} ({:.1} MB)]",
            size().await as f64 / 1048576.0
        )),
        e if LEGACY_DOC_EXTS.contains(&e) => Ok(format!(
            "[Document: {name} — text extraction not supported for .{e} format]"
        )),
        _ => read_text(platform, project, &rel)
            .await
            .map_err(|error| read_error(path, error)),
    }
}

fn read_error(path: &str, error: StoreError) -> String {
    match error {
        StoreError::NotFound(_) => format!("File does not exist: '{path}'"),
        other => format!(
            "Failed to read file '{path}' as text: {other} (likely binary, locked, or non-UTF-8)"
        ),
    }
}

/// Extracts the text of a source and saves it next to the file for later
/// reads (images are not extracted here).
pub async fn preprocess_file(platform: &Platform, path: &str) -> Res<String> {
    let (project, rel) = resolve(platform, path)?;
    let ext = extension_of(&rel);
    let text = match ext.as_str() {
        "org" => read_text(platform, project, &rel)
            .await
            .map(|content| org_to_markdown(&content))
            .map_err(|error| read_error(path, error))?,
        e if needs_parser(e) => extract_with_parser(platform, project, &rel, e, false).await?,
        _ => return Ok("no preprocessing needed".to_string()),
    };
    write_cache(platform, project, &rel, &text).await?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use llm_wiki_storage::{MemoryStore, ProjectLocator};

    use super::*;

    struct Fixture {
        platform: Platform,
        root: String,
    }

    fn fixture() -> Fixture {
        let platform = Platform {
            files: Arc::new(MemoryStore::new()),
            streams: None,
            locator: ProjectLocator::default(),
        };
        let root = platform
            .locator
            .project_path(ProjectId(uuid::Uuid::new_v4()));
        Fixture { platform, root }
    }

    impl Fixture {
        fn at(&self, rel: &str) -> String {
            format!("{}/{rel}", self.root)
        }

        async fn put(&self, rel: &str, contents: &str) {
            write_file(&self.platform, &self.at(rel), contents.to_string())
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn text_round_trips_for_pages_and_for_sources() {
        let f = fixture();
        f.put("wiki/concepts/a.md", "# A").await;
        f.put("raw/sources/notes.txt", "plain").await;
        assert_eq!(
            read_file(&f.platform, &f.at("wiki/concepts/a.md"), None)
                .await
                .unwrap(),
            "# A"
        );
        assert_eq!(
            read_file(&f.platform, &f.at("raw/sources/notes.txt"), None)
                .await
                .unwrap(),
            "plain"
        );
        let err = read_file(&f.platform, &f.at("wiki/missing.md"), None)
            .await
            .unwrap_err();
        assert!(err.starts_with("File does not exist"), "{err}");
    }

    #[tokio::test]
    async fn host_paths_never_reach_the_disk() {
        let f = fixture();
        for path in ["/etc/passwd", "relative/path", "/projects/not-a-uuid/x"] {
            assert!(read_file(&f.platform, path, None).await.is_err(), "{path}");
            assert!(
                write_file(&f.platform, path, String::new()).await.is_err(),
                "{path}"
            );
            assert!(
                list_directory(&f.platform, path, None, None).await.is_err(),
                "{path}"
            );
            assert!(delete_file(&f.platform, path).await.is_err(), "{path}");
            assert!(file_exists(&f.platform, path).await.is_err(), "{path}");
        }
    }

    #[tokio::test]
    async fn binary_files_round_trip_and_pages_must_be_text() {
        use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
        let f = fixture();
        let png = B64.encode([0x89, b'P', b'N', b'G', 0, 1, 2]);
        write_file_base64(&f.platform, &f.at("wiki/media/pic.png"), png.clone())
            .await
            .unwrap();
        let read = read_file_as_base64(&f.platform, &f.at("wiki/media/pic.png"))
            .await
            .unwrap();
        assert_eq!((read.base64, read.mime_type.as_str()), (png, "image/png"));

        let bad = B64.encode([0xff, 0xfe]);
        let err = write_file_base64(&f.platform, &f.at("wiki/a.md"), bad)
            .await
            .unwrap_err();
        assert!(err.contains("must be text"), "{err}");
        // Text sent as base64 to a page is stored as a page.
        write_file_base64(&f.platform, &f.at("wiki/b.md"), B64.encode("hi"))
            .await
            .unwrap();
        assert_eq!(
            read_file(&f.platform, &f.at("wiki/b.md"), None)
                .await
                .unwrap(),
            "hi"
        );
    }

    #[tokio::test]
    async fn a_listing_is_a_sorted_tree_without_dot_entries() {
        let f = fixture();
        f.put("wiki/b.md", "b").await;
        f.put("wiki/a.md", "a").await;
        f.put("wiki/sub/deep/c.md", "c").await;
        f.put("wiki/.hidden/x.md", "x").await;
        f.put("raw/sources/.claude/skill.md", "s").await;

        let tree = list_directory(&f.platform, &f.at("wiki"), None, None)
            .await
            .unwrap();
        let names: Vec<_> = tree.iter().map(|n| (n.name.as_str(), n.is_dir)).collect();
        assert_eq!(names, [("sub", true), ("a.md", false), ("b.md", false)]);
        assert_eq!(tree[0].path, f.at("wiki/sub"));
        let deep = &tree[0].children.as_ref().unwrap()[0];
        assert_eq!(deep.path, f.at("wiki/sub/deep"));
        assert_eq!(
            deep.children.as_ref().unwrap()[0].path,
            f.at("wiki/sub/deep/c.md")
        );

        // Depth limits how many levels are returned.
        let shallow = list_directory(&f.platform, &f.at("wiki"), None, Some(1))
            .await
            .unwrap();
        assert!(shallow[0].children.is_none());

        // Dot entries only when asked, which `raw/sources` does.
        let sources = list_directory(&f.platform, &f.at("raw/sources"), Some(true), None)
            .await
            .unwrap();
        assert_eq!(sources[0].name, ".claude");
        assert!(
            list_directory(&f.platform, &f.at("raw/sources"), None, None)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn an_empty_directory_lists_as_empty_but_a_file_is_not_a_directory() {
        let f = fixture();
        f.put("wiki/a.md", "a").await;
        assert!(
            list_directory(&f.platform, &f.at("wiki/nothing"), None, None)
                .await
                .unwrap()
                .is_empty()
        );
        let err = list_directory(&f.platform, &f.at("wiki/a.md"), None, None)
            .await
            .unwrap_err();
        assert!(err.contains("not a directory"), "{err}");
        create_directory(&f.platform, &f.at("wiki/new"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn copy_delete_and_file_facts() {
        let f = fixture();
        f.put("wiki/a.md", "hello").await;
        f.put("wiki/dir/b.md", "bee").await;
        f.put("wiki/dir/.secret", "no").await;

        copy_file(&f.platform, &f.at("wiki/a.md"), &f.at("raw/sources/a.md"))
            .await
            .unwrap();
        assert!(file_exists(&f.platform, &f.at("raw/sources/a.md"))
            .await
            .unwrap());
        assert_eq!(
            get_file_size(&f.platform, &f.at("wiki/a.md"))
                .await
                .unwrap(),
            5
        );
        assert_eq!(
            get_file_md5(&f.platform, &f.at("wiki/a.md")).await.unwrap(),
            "5d41402abc4b2a76b9719d911017c592"
        );
        assert!(
            get_file_modified_time(&f.platform, &f.at("wiki/a.md"))
                .await
                .unwrap()
                > 0
        );

        let copied = copy_directory(&f.platform, &f.at("wiki/dir"), &f.at("raw/copy"))
            .await
            .unwrap();
        assert_eq!(copied, [f.at("raw/copy/b.md")]);
        let err = copy_directory(&f.platform, &f.at("wiki/a.md"), &f.at("raw/x"))
            .await
            .unwrap_err();
        assert!(err.contains("not a directory"), "{err}");

        delete_file(&f.platform, &f.at("wiki/dir")).await.unwrap();
        assert!(!file_exists(&f.platform, &f.at("wiki/dir/b.md"))
            .await
            .unwrap());
        assert!(delete_file(&f.platform, &f.at("wiki/dir")).await.is_err());
        assert!(get_file_size(&f.platform, &f.at("wiki/gone.md"))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn files_without_text_read_as_a_description() {
        let f = fixture();
        write_file_base64(&f.platform, &f.at("raw/sources/p.png"), "AAAA".into())
            .await
            .unwrap();
        write_file_base64(&f.platform, &f.at("raw/sources/m.mp4"), "AAAA".into())
            .await
            .unwrap();
        write_file_base64(&f.platform, &f.at("raw/sources/n.numbers"), "AAAA".into())
            .await
            .unwrap();
        let text = |name: &'static str| {
            let path = f.at(name);
            let platform = &f.platform;
            async move { read_file(platform, &path, None).await.unwrap() }
        };
        assert_eq!(text("raw/sources/p.png").await, "[Image: p.png (0.0 KB)]");
        assert_eq!(text("raw/sources/m.mp4").await, "[Media: m.mp4 (0.0 MB)]");
        assert!(text("raw/sources/n.numbers")
            .await
            .starts_with("[Document: n.numbers"));
    }

    #[tokio::test]
    async fn preprocessing_caches_text_until_the_source_changes() {
        let f = fixture();
        let org = f.at("raw/sources/notes.org");
        f.put("raw/sources/notes.org", "* Heading\nbody").await;
        let first = preprocess_file(&f.platform, &org).await.unwrap();
        assert!(first.contains("Heading"), "{first}");
        assert_eq!(
            preprocess_file(&f.platform, &f.at("raw/sources/a.txt"))
                .await
                .unwrap(),
            "no preprocessing needed"
        );
        // The cache sits beside the source, hidden from listings.
        assert!(
            file_exists(&f.platform, &f.at("raw/sources/.cache/notes.org.txt"))
                .await
                .unwrap()
        );

        // Poison the cache: a read returns it while it is the newer file.
        tokio::time::sleep(Duration::from_millis(5)).await;
        f.put("raw/sources/.cache/notes.org.txt", "from cache")
            .await;
        assert_eq!(
            read_file(&f.platform, &org, None).await.unwrap(),
            "from cache"
        );

        // Editing the source makes the cache stale and it is read afresh.
        tokio::time::sleep(Duration::from_millis(5)).await;
        f.put("raw/sources/notes.org", "* Changed\nbody").await;
        let after = read_file(&f.platform, &org, None).await.unwrap();
        assert!(after.contains("Changed"), "{after}");
    }
}
