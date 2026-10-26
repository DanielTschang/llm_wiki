//! Behaviour every [`ProjectFiles`] store must have. Run against the memory
//! store always and against Postgres when `LLM_WIKI_TEST_DATABASE_URL` is set,
//! so the two cannot drift apart. Paths stay under `wiki/` with text bodies
//! because that is the part both stores serve.

use async_trait::async_trait;

use crate::store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
use crate::{RelPath, StoreError};

#[async_trait]
pub(crate) trait Harness: Send + Sync {
    fn store(&self) -> &dyn ProjectFiles;
    /// A Project no other test is using.
    async fn new_project(&self) -> ProjectId;
}

fn rel(path: &str) -> RelPath {
    RelPath::parse(path).unwrap()
}

fn text(content: &str) -> Body {
    Body::Text(content.to_string())
}

fn ctx() -> WriteCtx {
    WriteCtx::new("test")
}

async fn put(h: &dyn Harness, p: ProjectId, path: &str, content: &str) -> Meta {
    h.store()
        .write(p, &rel(path), text(content), Precondition::Any, &ctx())
        .await
        .unwrap()
}

pub(crate) async fn write_then_read_round_trips_and_versions_count_up(h: &dyn Harness) {
    let p = h.new_project().await;
    assert_eq!(put(h, p, "wiki/a.md", "one").await.version, Some(1));
    assert_eq!(put(h, p, "wiki/a.md", "two").await.version, Some(2));
    let (body, meta) = h.store().read(p, &rel("wiki/a.md")).await.unwrap();
    assert_eq!(body, text("two"));
    assert_eq!(meta.version, Some(2));
    assert_eq!(meta.size, 3);
}

pub(crate) async fn size_counts_bytes_not_characters(h: &dyn Harness) {
    let p = h.new_project().await;
    let meta = put(h, p, "wiki/a.md", "知識").await;
    assert_eq!(meta.size, 6);
}

pub(crate) async fn projects_are_isolated(h: &dyn Harness) {
    let (a, b) = (h.new_project().await, h.new_project().await);
    put(h, a, "wiki/a.md", "secret").await;
    assert!(matches!(
        h.store().read(b, &rel("wiki/a.md")).await,
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
            &rel("wiki/a.md"),
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
            &rel("wiki/a.md"),
            text("clobber"),
            Precondition::CreateOnly,
            &agent,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::AlreadyExists(_)));
    assert_eq!(
        h.store().read(p, &rel("wiki/a.md")).await.unwrap().0,
        text("kept")
    );
}

pub(crate) async fn a_stale_writer_gets_a_version_conflict(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/a.md", "v1").await;
    h.store()
        .write(
            p,
            &rel("wiki/a.md"),
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
            &rel("wiki/a.md"),
            text("stale"),
            Precondition::Version(1),
            &ctx(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        err,
        StoreError::VersionConflict {
            path: "wiki/a.md".into(),
            current: 2
        }
    );
    assert_eq!(
        h.store().read(p, &rel("wiki/a.md")).await.unwrap().0,
        text("mine")
    );
}

pub(crate) async fn versioned_write_to_a_missing_file_is_not_found(h: &dyn Harness) {
    let p = h.new_project().await;
    let err = h
        .store()
        .write(
            p,
            &rel("wiki/a.md"),
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
    put(h, p, "wiki/entities/a.md", "a").await;
    let dir = h
        .store()
        .stat(p, &rel("wiki/entities"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dir.kind, EntryKind::Dir);
    h.store()
        .delete(p, &rel("wiki/entities/a.md"), &ctx())
        .await
        .unwrap();
    assert_eq!(
        h.store().stat(p, &rel("wiki/entities")).await.unwrap(),
        None
    );
    assert_eq!(h.store().stat(p, &rel("wiki")).await.unwrap(), None);
}

pub(crate) async fn list_returns_direct_children_with_synthesised_directories(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/index.md", "i").await;
    put(h, p, "wiki/entities/a.md", "a").await;
    put(h, p, "wiki/entities/b.md", "b").await;
    put(h, p, "wiki2/x.md", "x").await;
    let children: Vec<_> = h
        .store()
        .list(p, &rel("wiki"), false)
        .await
        .unwrap()
        .iter()
        .map(|m| (m.path.to_string(), m.kind))
        .collect();
    assert_eq!(
        children,
        vec![
            ("wiki/entities".to_string(), EntryKind::Dir),
            ("wiki/index.md".to_string(), EntryKind::File),
        ]
    );
    let all = h.store().list(p, &rel("wiki"), true).await.unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.iter().all(|m| m.kind == EntryKind::File));
    let top: Vec<_> = h
        .store()
        .list(p, &RelPath::root(), false)
        .await
        .unwrap()
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(top, ["wiki", "wiki2"]);
}

pub(crate) async fn delete_removes_a_whole_subtree_and_counts_it(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/old/a.md", "a").await;
    put(h, p, "wiki/old/sub/b.md", "b").await;
    put(h, p, "wiki/other.md", "c").await;
    assert_eq!(
        h.store().delete(p, &rel("wiki/old"), &ctx()).await.unwrap(),
        2
    );
    assert!(h
        .store()
        .stat(p, &rel("wiki/other.md"))
        .await
        .unwrap()
        .is_some());
    assert!(matches!(
        h.store().delete(p, &rel("wiki/old"), &ctx()).await,
        Err(StoreError::NotFound(_))
    ));
}

pub(crate) async fn a_deleted_path_can_be_created_again_from_version_one(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/a.md", "first life").await;
    put(h, p, "wiki/a.md", "edited").await;
    h.store()
        .delete(p, &rel("wiki/a.md"), &ctx())
        .await
        .unwrap();
    let meta = h
        .store()
        .write(
            p,
            &rel("wiki/a.md"),
            text("second life"),
            Precondition::CreateOnly,
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(meta.version, Some(1));
    assert_eq!(
        h.store().read(p, &rel("wiki/a.md")).await.unwrap().0,
        text("second life")
    );
}

pub(crate) async fn rename_moves_a_directory_and_refuses_an_occupied_destination(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/old/a.md", "a").await;
    put(h, p, "wiki/old/deep/b.md", "b").await;
    put(h, p, "wiki/taken/c.md", "c").await;
    assert!(matches!(
        h.store()
            .rename(p, &rel("wiki/old"), &rel("wiki/taken"), &ctx())
            .await,
        Err(StoreError::AlreadyExists(_))
    ));
    assert!(matches!(
        h.store()
            .rename(p, &rel("wiki/old"), &rel("wiki/old/inside"), &ctx())
            .await,
        Err(StoreError::InvalidPath { .. })
    ));
    h.store()
        .rename(p, &rel("wiki/old"), &rel("wiki/new"), &ctx())
        .await
        .unwrap();
    let (body, meta) = h.store().read(p, &rel("wiki/new/deep/b.md")).await.unwrap();
    assert_eq!(body, text("b"));
    assert_eq!(meta.version, Some(2));
    assert_eq!(h.store().stat(p, &rel("wiki/old")).await.unwrap(), None);
    assert!(matches!(
        h.store()
            .rename(p, &rel("wiki/missing"), &rel("wiki/x"), &ctx())
            .await,
        Err(StoreError::NotFound(_))
    ));
}

pub(crate) async fn a_file_cannot_shadow_a_directory_or_sit_beneath_a_file(h: &dyn Harness) {
    let p = h.new_project().await;
    put(h, p, "wiki/entities/a.md", "a").await;
    assert!(h
        .store()
        .write(
            p,
            &rel("wiki/entities"),
            text("x"),
            Precondition::Any,
            &ctx()
        )
        .await
        .is_err());
    assert!(h
        .store()
        .write(
            p,
            &rel("wiki/entities/a.md/child"),
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
    put(h, p, "wiki/a_b/x.md", "mine").await;
    put(h, p, "wiki/axb/x.md", "not mine").await;
    put(h, p, "wiki/100%/x.md", "pct").await;
    put(h, p, "wiki/100z/x.md", "not pct").await;
    assert_eq!(
        h.store().delete(p, &rel("wiki/a_b"), &ctx()).await.unwrap(),
        1
    );
    assert_eq!(
        h.store()
            .delete(p, &rel("wiki/100%"), &ctx())
            .await
            .unwrap(),
        1
    );
    assert!(h
        .store()
        .stat(p, &rel("wiki/axb/x.md"))
        .await
        .unwrap()
        .is_some());
    assert!(h
        .store()
        .stat(p, &rel("wiki/100z/x.md"))
        .await
        .unwrap()
        .is_some());
    let names: Vec<_> = h
        .store()
        .list(p, &rel("wiki"), false)
        .await
        .unwrap()
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(names, ["wiki/100z", "wiki/axb"]);
}

/// Expands to one `#[tokio::test]` per case. `$setup` is an expression of
/// type `Option<impl Harness>`; `None` skips (no database configured).
macro_rules! conformance_tests {
    ($setup:expr $(,)?) => {
        $crate::conformance::conformance_tests!(@cases $setup;
            write_then_read_round_trips_and_versions_count_up,
            size_counts_bytes_not_characters,
            projects_are_isolated,
            create_only_refuses_to_overwrite,
            a_stale_writer_gets_a_version_conflict,
            versioned_write_to_a_missing_file_is_not_found,
            directories_exist_only_while_they_hold_files,
            list_returns_direct_children_with_synthesised_directories,
            delete_removes_a_whole_subtree_and_counts_it,
            a_deleted_path_can_be_created_again_from_version_one,
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
