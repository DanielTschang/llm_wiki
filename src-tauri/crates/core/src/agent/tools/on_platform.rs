//! The agent tools that touch Project files, for a platform server: the same
//! tools and the same results, with pages, sources and the agent workspace
//! read from and written to the storage layer instead of the host disk.
//!
//! Path rules, size limits, graph ranking and snippet building are the ones
//! in the parent module; only where the files come from differs.

use llm_wiki_storage::{Body, EntryKind, Precondition, ProjectId, RelPath, StoreError, WriteCtx};
use serde_json::{json, Value};

use super::*;
use crate::commands::fs_platform::read_cache;
use crate::platform::Platform;

type Res<T> = Result<T, String>;

fn store_error(error: StoreError) -> String {
    error.to_string()
}

fn project_of(platform: &Platform, project_path: &str) -> Res<ProjectId> {
    let (project, root) = platform
        .locator
        .resolve(project_path)
        .map_err(store_error)?;
    if !root.is_root() {
        return Err(format!("not a project path: {project_path}"));
    }
    Ok(project)
}

fn rel_path(path: &str) -> Res<RelPath> {
    RelPath::parse(path).map_err(store_error)
}

fn agent_ctx() -> WriteCtx {
    WriteCtx::new("agent")
}

/// Runs a tool that touches Project files. `None` for tools that do not, so
/// the caller falls through to the shared implementation.
pub(super) async fn execute(
    platform: &Platform,
    name: &str,
    input: &Value,
    project_path: &str,
) -> Option<Res<Value>> {
    Some(match name {
        "wiki.write_page" => write_page(platform, input, project_path).await,
        "wiki.read_page" => read_page(platform, input, project_path).await,
        "workspace.write_file" => write_workspace(platform, input, project_path, false).await,
        "workspace.append_file" => write_workspace(platform, input, project_path, true).await,
        "source.search" => search_sources_in(platform, input, project_path).await,
        "graph.search" => search_graph_in(platform, input, project_path).await,
        "shell.exec" => Err("shell.exec is not available on the platform server".to_string()),
        _ => return None,
    })
}

/// A wiki page's text, with the same path rules as the disk tool.
pub(super) async fn read_page_text(
    platform: &Platform,
    project_path: &str,
    rel_path_in: &str,
) -> Res<String> {
    let rel = normalize_rel_path(rel_path_in);
    if !is_public_read_rel(&rel) || !rel.to_ascii_lowercase().starts_with("wiki/") {
        return Err("wiki.read_page path must stay under wiki/".to_string());
    }
    let project = project_of(platform, project_path)?;
    let (body, meta) = platform
        .files
        .read(project, &rel_path(&rel)?)
        .await
        .map_err(|err| format!("Failed to read page metadata: {err}"))?;
    if meta.size as usize > MAX_READ_PAGE_BYTES {
        return Err("wiki.read_page file is too large".to_string());
    }
    body.as_text()
        .map(str::to_string)
        .ok_or_else(|| "Failed to read wiki page: not valid UTF-8".to_string())
}

async fn wiki_sources(platform: &Platform, project_path: &str) -> Res<Vec<WikiPageSource>> {
    Ok(platform
        .wiki_markdown(project_path, MAX_GRAPH_SEARCH_FILES)
        .await?
        .into_iter()
        .map(|(path, content)| WikiPageSource {
            file_name: path.rsplit('/').next().unwrap_or_default().to_string(),
            path,
            content,
        })
        .collect())
}

async fn read_page(platform: &Platform, input: &Value, project_path: &str) -> Res<Value> {
    let path = input
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "wiki.read_page requires path".to_string())?;
    let content = read_page_text(platform, project_path, path).await?;
    let normalized_path = normalize_rel_path(path);
    let knowledge_context =
        knowledge_snapshot_from(wiki_sources(platform, project_path).await?, None)
            .1
            .remove(&normalized_path);
    Ok(json!({
        "path": path,
        "content": content,
        "knowledgeContext": knowledge_context,
    }))
}

async fn write_page(platform: &Platform, input: &Value, project_path: &str) -> Res<Value> {
    let path = input
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "wiki.write_page requires path".to_string())?;
    let content = input
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| "wiki.write_page requires content".to_string())?;
    let allow_overwrite = input
        .get("allowOverwrite")
        .or_else(|| input.get("allow_overwrite"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if content.len() > MAX_WRITE_PAGE_BYTES {
        return Err("wiki.write_page content is too large".to_string());
    }
    let rel = normalize_wiki_write_path(path)?;
    let target = rel_path(&rel)?;
    let project = project_of(platform, project_path)?;

    let previous = match platform.files.read(project, &target).await {
        Ok((body, _)) => Some(body),
        Err(StoreError::NotFound(_)) => None,
        Err(error) => return Err(format!("Failed to read existing wiki page: {error}")),
    };
    let existed_before = previous.is_some();
    // Create-only by default. Prompt injection in retrieved context must not be
    // able to silently truncate an existing wiki page.
    let refusal = || {
        "wiki.write_page refuses to overwrite an existing page without allowOverwrite=true"
            .to_string()
    };
    if existed_before && !allow_overwrite {
        return Err(refusal());
    }
    let precondition = if allow_overwrite {
        Precondition::Any
    } else {
        Precondition::CreateOnly
    };
    platform
        .files
        .write(
            project,
            &target,
            Body::Text(content.to_string()),
            precondition,
            &agent_ctx(),
        )
        .await
        .map_err(|error| match error {
            StoreError::AlreadyExists(_) => refusal(),
            other => format!("wiki.write_page failed: {other}"),
        })?;

    let persisted = platform
        .files
        .read(project, &target)
        .await
        .map_err(|err| format!("Failed to verify persisted wiki page: {err}"))?
        .0;
    if persisted.as_text() != Some(content) {
        let rollback = match &previous {
            Some(body) => platform
                .files
                .write(
                    project,
                    &target,
                    body.clone(),
                    Precondition::Any,
                    &agent_ctx(),
                )
                .await
                .map(|_| ()),
            None => platform
                .files
                .delete(project, &target, &agent_ctx())
                .await
                .map(|_| ()),
        };
        return Err(match rollback {
            Ok(()) => "Persisted wiki page did not match the requested content; the previous state was restored".to_string(),
            Err(err) => format!("Persisted wiki page did not match the requested content, and rollback failed: {err}"),
        });
    }

    let previous_content = previous
        .and_then(|body| body.as_text().map(str::to_string))
        .filter(|text| text.len() as u64 <= MAX_WORKSPACE_ROLLBACK_BYTES);
    serde_json::to_value(WikiWriteOutput {
        reference: AgentReference {
            title: extract_markdown_title(content).unwrap_or_else(|| {
                Path::new(&rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Wiki page")
                    .replace('-', " ")
            }),
            path: rel.clone(),
            kind: "wiki".to_string(),
            snippet: Some(trim_text(&collapse_markdown_preview(content), 500))
                .filter(|value| !value.trim().is_empty()),
            score: None,
            knowledge_context: None,
        },
        existed_before,
        previous_content,
    })
    .map_err(|err| format!("Failed to serialize wiki.write_page result: {err}"))
}

async fn write_workspace(
    platform: &Platform,
    input: &Value,
    project_path: &str,
    append: bool,
) -> Res<Value> {
    let tool = if append {
        "workspace.append_file"
    } else {
        "workspace.write_file"
    };
    let path = input
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{tool} requires path"))?;
    let content = input
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{tool} requires content"))?;
    if content.len() > MAX_WORKSPACE_WRITE_BYTES {
        return Err(format!("{tool} content is too large"));
    }
    let rel = normalize_workspace_write_path(path)
        .map_err(|err| err.replace("workspace.write_file", tool))?;
    let project = project_of(platform, project_path)?;
    let full = format!("{AGENT_WORKSPACE_DIR}/{rel}");
    let target = rel_path(&full)?;

    let previous = match platform.files.read(project, &target).await {
        Ok((body, _)) => Some(body.into_bytes()),
        Err(StoreError::NotFound(_)) => None,
        Err(error) => return Err(format!("{tool} failed: {error}")),
    };
    let previous_content = previous
        .as_ref()
        .filter(|bytes| bytes.len() as u64 <= MAX_WORKSPACE_ROLLBACK_BYTES)
        .and_then(|bytes| String::from_utf8(bytes.clone()).ok());
    let mut bytes = if append {
        previous.clone().unwrap_or_default()
    } else {
        Vec::new()
    };
    bytes.extend_from_slice(content.as_bytes());
    let total = bytes.len();
    platform
        .files
        .write(
            project,
            &target,
            Body::Bytes(bytes),
            Precondition::Any,
            &agent_ctx(),
        )
        .await
        .map_err(|err| format!("{tool} failed: {err}"))?;
    serde_json::to_value(WorkspaceWriteOutput {
        path: full,
        bytes: total,
        existed_before: previous.is_some(),
        previous_content,
    })
    .map_err(|err| format!("Failed to serialize {tool} result: {err}"))
}

async fn search_sources_in(platform: &Platform, input: &Value, project_path: &str) -> Res<Value> {
    let query = tool_query(input, "source.search")?.trim().to_string();
    if query.is_empty() {
        return Err("source.search query is required".to_string());
    }
    let top_k = tool_top_k(input);
    let project = project_of(platform, project_path)?;
    let root = rel_path("raw/sources")?;
    let lower_query = query.to_lowercase();
    let query_terms = source_query_terms(&lower_query);
    let mut refs = Vec::new();
    let mut seen_files = 0usize;
    for meta in platform
        .files
        .list(project, &root, true)
        .await
        .map_err(store_error)?
    {
        let rel = meta.path.to_string();
        if meta.kind != EntryKind::File || is_hidden_rel(&rel) {
            continue;
        }
        seen_files += 1;
        if seen_files > MAX_SOURCE_SEARCH_FILES {
            eprintln!("[Agent] source.search stopped after {MAX_SOURCE_SEARCH_FILES} files in {project_path}");
            break;
        }
        let Some(ext) = meta.path.extension().map(str::to_ascii_lowercase) else {
            continue;
        };
        let content = if TEXT_SOURCE_EXTENSIONS.contains(&ext.as_str()) {
            match platform.files.read(project, &meta.path).await {
                Ok((body, _)) => match body.as_text() {
                    Some(text) => text.to_string(),
                    None => continue,
                },
                Err(_) => continue,
            }
        } else if PARSED_SOURCE_EXTENSIONS.contains(&ext.as_str()) {
            let Some(cached) = read_cache(platform, project, &meta.path).await else {
                continue;
            };
            cached
        } else {
            continue;
        };
        let file_name = meta.path.file_name().unwrap_or(&rel).to_string();
        if let Some(reference) = match_source(rel, &file_name, &content, &lower_query, &query_terms)
        {
            refs.push(reference);
            if refs.len() >= top_k.clamp(1, 10) {
                break;
            }
        }
    }
    serde_json::to_value(refs)
        .map_err(|err| format!("Failed to serialize source.search result: {err}"))
}

async fn search_graph_in(platform: &Platform, input: &Value, project_path: &str) -> Res<Value> {
    let query = tool_query(input, "graph.search")?.trim().to_lowercase();
    let top_k = tool_top_k(input);
    let references = if query.is_empty() {
        Vec::new()
    } else {
        let (pages, contexts) =
            knowledge_snapshot_from(wiki_sources(platform, project_path).await?, Some(&query));
        rank_graph_pages(pages, contexts, top_k, |_, _| {})
    };
    serde_json::to_value(references)
        .map_err(|err| format!("Failed to serialize graph.search result: {err}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use llm_wiki_storage::{MemoryStore, ProjectLocator};

    use super::*;

    struct Project {
        platform: Platform,
        id: ProjectId,
        path: String,
    }

    fn project() -> Project {
        let locator = ProjectLocator::default();
        let id = ProjectId(uuid::Uuid::new_v4());
        Project {
            path: locator.project_path(id),
            id,
            platform: Platform {
                vectors: None,
                files: Arc::new(MemoryStore::new()),
                streams: None,
                locator,
            },
        }
    }

    impl Project {
        async fn put(&self, rel: &str, content: &str) {
            self.platform
                .files
                .write(
                    self.id,
                    &RelPath::parse(rel).unwrap(),
                    Body::Text(content.to_string()),
                    Precondition::Any,
                    &WriteCtx::new("test"),
                )
                .await
                .unwrap();
        }

        async fn text(&self, rel: &str) -> Option<String> {
            self.platform.read_text(&self.path, rel).await
        }

        async fn run(&self, tool: &str, input: Value) -> Res<Value> {
            execute(&self.platform, tool, &input, &self.path)
                .await
                .expect("a platform tool")
        }
    }

    #[tokio::test]
    async fn a_page_write_creates_but_will_not_overwrite_unless_asked() {
        let p = project();
        let created = p
            .run(
                "wiki.write_page",
                json!({ "path": "wiki/concepts/a.md", "content": "# Alpha\nbody" }),
            )
            .await
            .unwrap();
        assert_eq!(created["path"], "wiki/concepts/a.md");
        assert_eq!(created["title"], "Alpha");
        assert_eq!(created["existedBefore"], false);
        assert_eq!(
            p.text("wiki/concepts/a.md").await.as_deref(),
            Some("# Alpha\nbody")
        );

        let refused = p
            .run(
                "wiki.write_page",
                json!({ "path": "wiki/concepts/a.md", "content": "# Hijacked" }),
            )
            .await
            .unwrap_err();
        assert!(refused.contains("refuses to overwrite"), "{refused}");
        assert_eq!(
            p.text("wiki/concepts/a.md").await.as_deref(),
            Some("# Alpha\nbody")
        );

        let replaced = p
            .run(
                "wiki.write_page",
                json!({ "path": "wiki/concepts/a.md", "content": "# Alpha 2", "allowOverwrite": true }),
            )
            .await
            .unwrap();
        assert_eq!(replaced["existedBefore"], true);
        assert_eq!(replaced["previousContent"], "# Alpha\nbody");
    }

    #[tokio::test]
    async fn a_page_write_keeps_to_markdown_under_wiki() {
        let p = project();
        for path in [
            "raw/sources/a.md",
            "wiki/a.txt",
            "wiki/.hidden/a.md",
            "/etc/x.md",
            "../a.md",
        ] {
            let err = p
                .run("wiki.write_page", json!({ "path": path, "content": "x" }))
                .await
                .unwrap_err();
            assert!(err.contains("wiki.write_page"), "{path}: {err}");
        }
        assert!(p.text("raw/sources/a.md").await.is_none());
    }

    #[tokio::test]
    async fn reading_a_page_returns_its_links_and_refuses_other_areas() {
        let p = project();
        p.put("wiki/a.md", "# A\nsee [[b]]").await;
        p.put("wiki/b.md", "# B\nback to [[a]]").await;
        p.put("raw/sources/s.md", "source").await;
        let read = p
            .run("wiki.read_page", json!({ "path": "wiki/a.md" }))
            .await
            .unwrap();
        assert_eq!(read["content"], "# A\nsee [[b]]");
        assert_eq!(read["knowledgeContext"]["outgoingLinks"], json!(["b"]));
        assert_eq!(read["knowledgeContext"]["backlinks"], json!(["wiki/b.md"]));

        for path in ["raw/sources/s.md", "../x.md", ".llm-wiki/state.md"] {
            assert!(
                p.run("wiki.read_page", json!({ "path": path }))
                    .await
                    .is_err(),
                "{path}"
            );
        }
        assert!(read_page_text(&p.platform, "/etc", "wiki/a.md")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn workspace_files_are_written_then_appended_to() {
        let p = project();
        let first = p
            .run(
                "workspace.write_file",
                json!({ "path": "report.html", "content": "<h1>one</h1>" }),
            )
            .await
            .unwrap();
        assert_eq!(first["path"], "agent-workspace/report.html");
        assert_eq!(first["bytes"], 12);
        assert_eq!(first["existedBefore"], false);

        let appended = p
            .run(
                "workspace.append_file",
                json!({ "path": "report.html", "content": "<p>two</p>" }),
            )
            .await
            .unwrap();
        assert_eq!(appended["bytes"], 22);
        assert_eq!(appended["existedBefore"], true);
        assert_eq!(appended["previousContent"], "<h1>one</h1>");
        assert_eq!(
            p.text("agent-workspace/report.html").await.as_deref(),
            Some("<h1>one</h1><p>two</p>")
        );

        for path in ["../escape.txt", ".hidden"] {
            assert!(
                p.run(
                    "workspace.write_file",
                    json!({ "path": path, "content": "x" })
                )
                .await
                .is_err(),
                "{path}"
            );
        }
        // A leading slash is dropped, as on disk: the file stays in the workspace.
        let rooted = p
            .run(
                "workspace.write_file",
                json!({ "path": "/abs.txt", "content": "x" }),
            )
            .await
            .unwrap();
        assert_eq!(rooted["path"], "agent-workspace/abs.txt");
    }

    #[tokio::test]
    async fn source_search_reads_text_files_and_fresh_extraction_caches_only() {
        let p = project();
        p.put(
            "raw/sources/notes.txt",
            "Quarterly revenue grew in the north",
        )
        .await;
        p.put("raw/sources/other.txt", "nothing relevant").await;
        p.put("raw/sources/.hidden.txt", "revenue in a hidden file")
            .await;
        // A PDF is searched through the text its preprocessing saved.
        p.platform
            .files
            .write(
                p.id,
                &RelPath::parse("raw/sources/deck.pdf").unwrap(),
                Body::Bytes(vec![0x25, 0x50, 0x44, 0x46]),
                Precondition::Any,
                &WriteCtx::new("test"),
            )
            .await
            .unwrap();
        let search = |query: &'static str| {
            let p = &p;
            async move {
                p.run("source.search", json!({ "query": query }))
                    .await
                    .unwrap()
            }
        };
        let hits = search("revenue").await;
        let paths: Vec<_> = hits
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["raw/sources/notes.txt"]);

        assert_eq!(search("margin").await, json!([]));
        tokio::time::sleep(Duration::from_millis(5)).await;
        p.put("raw/sources/.cache/deck.pdf.txt", "The margin improved")
            .await;
        let hits = search("margin").await;
        assert_eq!(hits[0]["path"], "raw/sources/deck.pdf");
        assert_eq!(hits[0]["kind"], "source");

        assert!(p
            .run("source.search", json!({ "query": "  " }))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn graph_search_returns_the_match_and_its_neighbours() {
        let p = project();
        p.put("wiki/solar.md", "# Solar power\nsee [[grid]]").await;
        p.put("wiki/grid.md", "# Grid\nlinks back [[solar]]").await;
        p.put("wiki/unrelated.md", "# Cooking\nrecipes").await;
        let hits = p
            .run("graph.search", json!({ "query": "power" }))
            .await
            .unwrap();
        let paths: Vec<_> = hits
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["wiki/solar.md", "wiki/grid.md"]);
        assert_eq!(hits[0]["kind"], "graph");
        assert_eq!(
            p.run("graph.search", json!({ "query": "zzz-no-match" }))
                .await
                .unwrap(),
            json!([])
        );
    }

    #[tokio::test]
    async fn the_shell_is_not_available_and_other_tools_fall_through() {
        let p = project();
        let err = p
            .run("shell.exec", json!({ "command": "ls" }))
            .await
            .unwrap_err();
        assert!(err.contains("not available"), "{err}");
        for tool in [
            "wiki.search",
            "web.search",
            "anytxt.search",
            "deep_research.run",
        ] {
            assert!(
                execute(&p.platform, tool, &json!({}), &p.path)
                    .await
                    .is_none(),
                "{tool}"
            );
        }
    }
}
