//! Behaviour every [`ProjectFiles`] store must have. Run against the memory
//! store always, and against the Postgres page store and blob store when
//! `LLM_WIKI_TEST_DATABASE_URL` is set, so they cannot drift apart. Each
//! harness names the folder it serves (`root`) and whether its files carry
//! versions.

use async_trait::async_trait;

use crate::store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
use crate::{RelPath, StoreError};

#[async_trait]
pub(crate) trait Harness: Send + Sync {
    fn store(&self) -> &dyn ProjectFiles;
    /// A Project no other test is using.
    async fn new_project(&self) -> ProjectId;
    /// The folder this store serves, e.g. `wiki` or `raw/sources`.
    fn root(&self) -> &'static str;
    /// Whether files have a version (pages do, blobs do not).
    fn versioned(&self) -> bool;
}

fn at(h: &dyn Harness, tail: &str) -> RelPath {
    RelPath::parse(&format!("{}/{tail}", h.root())).unwrap()
}

fn root(h: &dyn Harness) -> RelPath {
    RelPath::parse(h.root()).unwrap()
}

fn text(content: &str) -> Body {
    Body::Text(content.to_string())
}

fn ctx() -> WriteCtx {
    WriteCtx::new("test")
}

async fn put(h: &dyn Harness, p: ProjectId, tail: &str, content: &str) -> Meta {
    h.store()
        .write(p, &at(h, tail), text(content), Precondition::Any, &ctx())
        .await
        .unwrap()
}

async fn content_of(h: &dyn Harness, p: ProjectId, tail: &str) -> String {
    let (body, _) = h.store().read(p, &at(h, tail)).await.unwrap();
    body.as_text().expect("test content is UTF-8").to_string()
}

pub(crate) async fn write_then_read_round_trips(h: &dyn Harness) {
    let p = h.new_project().await;
    let first = put(h, p, "a.md", "one").await;
    let second = put(h, p, "a.md", "two").await;
    let (body, meta) = h.store().read(p, &at(h, "a.md")).await.unwrap();
    assert_eq!(body.as_text(), Some("two"));
    assert_eq!(meta.size, 3);
    if h.versioned() {
        assert_eq!(
            (first.version, second.version, meta.version),
            (Some(1), Some(2), Some(2))
        );
    } else {
        assert_eq!(meta.version, None);
    }
}

pub(crate) async fn size_counts_bytes_not_characters(h: &dyn Harness) {
    let p = h.new_project().await;
    assert_eq!(put(h, p, "a.md", "知識").await.size, 6);
}

pub(crate) async fn projects_are_isolated(h: &dyn Harness) {
    let (a, b) = (h.new_project().await, h.new_project().await);
    put(h, a, "a.md", "secret").await;
    assert!(matches!(
        h.store().read(b, &at(h, "a.md")).await,
        Err(StoreError::NotFound(_))
    ));
    assert!(h
        .store()
        .list(b, &RelPath::root(), true)
        .await
        .unwrap()
        .is_empty());
}

pub(crate) async fn create_only_refuses_to_overwrite(h: &dyn Harness) {
    let p = h.new_project().await;
    let agent = WriteCtx::new("agent");
    h.store()
        .write(
            p,
            &at(h, "a.md"),
            text("kept"),
            Precondition::CreateOnly,
            &agent,
        )
        .await
        .unwrap();
    let err = h
        .store()
        .write(
            p,
            &at(h, "a.md"),
            text("clobber"),
            Precondition::CreateOnly,
            &agent,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::AlreadyExists(_)));
    assert_eq!(content_of(h, p, "a.md").await, "kept");
}

pub(crate) async fn a_stale_writer_gets_a_version_conflict(h: &dyn Harness) {
    if !h.versioned() {
        return;
    }
    let p = h.new_project().await;
    put(h, p, "a.md", "v1").await;
    h.store()
        .write(
            p,
            &at(h, "a.md"),
            text("mine"),
            Precondition::Version(1),
            &ctx(),
        )
        .await
        .unwrap();
    let err = h
        .store()
        .write(
            p,
            &at(h, "a.md"),
            text("stale"),
            Precondition::Version(1),
            &ctx(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        StoreError::VersionConflict {
            path: at(h, "a.md").to_string(),
            current: 2
        }
    );
    assert_eq!(content_of(h, p, "a.md").await, "mine");
}

pub(crate) async fn versioned_write_to_a_missing_file_is_not_found(h: &dyn Harness) {
    if !h.versioned() {
        return;
    }
    let p = h.new_project().await;
    let err = h
        .store()
        .write(
            p,
            &at(h, "a.md"),
            text("x"),
            Precondition::Version(1),
            &ctx(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::NotFound(_)));
}

pub(crate) async fn directories_exist_only_while_they_hold_files(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "entities/a.md", "a").await;
    let dir = h
        .store()
        .stat(p, &at(h, "entities"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dir.kind, EntryKind::Dir);
    h.store()
        .delete(p, &at(h, "entities/a.md"), &ctx())
        .await
        .unwrap();
    assert_eq!(h.store().stat(p, &at(h, "entities")).await.unwrap(), None);
    assert_eq!(h.store().stat(p, &root(h)).await.unwrap(), None);
}

pub(crate) async fn list_returns_direct_children_with_synthesised_directories(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "index.md", "i").await;
    put(h, p, "entities/a.md", "a").await;
    put(h, p, "entities/b.md", "b").await;
    let sibling = RelPath::parse(&format!("{}2/x.md", h.root())).unwrap();
    h.store()
        .write(p, &sibling, text("x"), Precondition::Any, &ctx())
        .await
        .unwrap();

    let children: Vec<_> = h
        .store()
        .list(p, &root(h), false)
        .await
        .unwrap()
        .iter()
        .map(|m| (m.path.to_string(), m.kind))
        .collect();
    assert_eq!(
        children,
        vec![
            (at(h, "entities").to_string(), EntryKind::Dir),
            (at(h, "index.md").to_string(), EntryKind::File),
        ]
    );
    let all = h.store().list(p, &root(h), true).await.unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|m| m.kind == EntryKind::File));

    // The sibling folder `<root>2` is not inside `<root>`.
    let parent = root(h).parent().unwrap();
    let names: Vec<_> = h
        .store()
        .list(p, &parent, false)
        .await
        .unwrap()
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(names, [h.root().to_string(), format!("{}2", h.root())]);
}

pub(crate) async fn delete_removes_a_whole_subtree_and_counts_it(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "old/a.md", "a").await;
    put(h, p, "old/sub/b.md", "b").await;
    put(h, p, "other.md", "c").await;
    assert_eq!(h.store().delete(p, &at(h, "old"), &ctx()).await.unwrap(), 2);
    assert!(h
        .store()
        .stat(p, &at(h, "other.md"))
        .await
        .unwrap()
        .is_some());
    assert!(matches!(
        h.store().delete(p, &at(h, "old"), &ctx()).await,
        Err(StoreError::NotFound(_))
    ));
}

pub(crate) async fn a_deleted_path_can_be_created_again(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "a.md", "first life").await;
    put(h, p, "a.md", "edited").await;
    h.store().delete(p, &at(h, "a.md"), &ctx()).await.unwrap();
    let meta = h
        .store()
        .write(
            p,
            &at(h, "a.md"),
            text("second life"),
            Precondition::CreateOnly,
            &ctx(),
        )
        .await
        .unwrap();
    if h.versioned() {
        assert_eq!(meta.version, Some(1));
    }
    assert_eq!(content_of(h, p, "a.md").await, "second life");
}

pub(crate) async fn rename_moves_a_directory_and_refuses_an_occupied_destination(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "old/a.md", "a").await;
    put(h, p, "old/deep/b.md", "b").await;
    put(h, p, "taken/c.md", "c").await;
    assert!(matches!(
        h.store()
            .rename(p, &at(h, "old"), &at(h, "taken"), &ctx())
            .await,
        Err(StoreError::AlreadyExists(_))
    ));
    assert!(matches!(
        h.store()
            .rename(p, &at(h, "old"), &at(h, "old/inside"), &ctx())
            .await,
        Err(StoreError::InvalidPath { .. })
    ));
    h.store()
        .rename(p, &at(h, "old"), &at(h, "new"), &ctx())
        .await
        .unwrap();
    let (body, meta) = h.store().read(p, &at(h, "new/deep/b.md")).await.unwrap();
    assert_eq!(body.as_text(), Some("b"));
    if h.versioned() {
        assert_eq!(meta.version, Some(2));
    }
    assert_eq!(h.store().stat(p, &at(h, "old")).await.unwrap(), None);
    assert!(matches!(
        h.store()
            .rename(p, &at(h, "missing"), &at(h, "x"), &ctx())
            .await,
        Err(StoreError::NotFound(_))
    ));
}

pub(crate) async fn a_file_cannot_shadow_a_directory_or_sit_beneath_a_file(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "entities/a.md", "a").await;
    assert!(h
        .store()
        .write(p, &at(h, "entities"), text("x"), Precondition::Any, &ctx())
        .await
        .is_err());
    assert!(h
        .store()
        .write(
            p,
            &at(h, "entities/a.md/child"),
            text("x"),
            Precondition::Any,
            &ctx()
        )
        .await
        .is_err());
}

/// `_` and `%` are wildcards in SQL `LIKE`; in a path they are ordinary characters.
pub(crate) async fn like_wildcards_in_names_are_literal(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "a_b/x.md", "mine").await;
    put(h, p, "axb/x.md", "not mine").await;
    put(h, p, "100%/x.md", "pct").await;
    put(h, p, "100z/x.md", "not pct").await;
    assert_eq!(h.store().delete(p, &at(h, "a_b"), &ctx()).await.unwrap(), 1);
    assert_eq!(
        h.store().delete(p, &at(h, "100%"), &ctx()).await.unwrap(),
        1
    );
    assert!(h
        .store()
        .stat(p, &at(h, "axb/x.md"))
        .await
        .unwrap()
        .is_some());
    assert!(h
        .store()
        .stat(p, &at(h, "100z/x.md"))
        .await
        .unwrap()
        .is_some());
    let names: Vec<_> = h
        .store()
        .list(p, &root(h), false)
        .await
        .unwrap()
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(names, [at(h, "100z").to_string(), at(h, "axb").to_string()]);
}

/// Expands to one `#[tokio::test]` per case. `$setup` is an expression of
/// type `Option<impl Harness>`; `None` skips (no database configured).
macro_rules! conformance_tests {
    ($setup:expr $(,)?) => {
        $crate::conformance::conformance_tests!(@cases $setup;
            write_then_read_round_trips,
            size_counts_bytes_not_characters,
            projects_are_isolated,
            create_only_refuses_to_overwrite,
            a_stale_writer_gets_a_version_conflict,
            versioned_write_to_a_missing_file_is_not_found,
            directories_exist_only_while_they_hold_files,
            list_returns_direct_children_with_synthesised_directories,
            delete_removes_a_whole_subtree_and_counts_it,
            a_deleted_path_can_be_created_again,
            rename_moves_a_directory_and_refuses_an_occupied_destination,
            a_file_cannot_shadow_a_directory_or_sit_beneath_a_file,
            like_wildcards_in_names_are_literal,
        );
    };
    (@cases $setup:expr; $($case:ident),+ $(,)?) => {
        $(
            #[tokio::test(flavor = "multi_thread")]
            async fn $case() {
                let Some(harness) = $setup else {
                    eprintln!("skipped: LLM_WIKI_TEST_DATABASE_URL is not set");
                    return;
                };
                conformance::$case(&harness).await;
            }
        )+
    };
}
pub(crate) use conformance_tests;
