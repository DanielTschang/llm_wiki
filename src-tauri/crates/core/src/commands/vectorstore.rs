//! Chunk embeddings for semantic search, served from the platform's vector
//! store (pgvector). The commands keep the argument and result shapes the
//! frontend already uses: a page id is the page's path under `wiki/` without
//! `.md`, and a score is `1 / (1 + distance)`.

use serde::{Deserialize, Serialize};

use crate::panic_guard::run_guarded_async;

/// Per-chunk search result. Surfaces the matching chunk's text and heading
/// path so the chat UI can show "matched in this section"; the frontend
/// groups chunks by `page_id`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkSearchResult {
    pub chunk_id: String,
    pub page_id: String,
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    pub score: f32,
}

/// Input row for `vector_upsert_chunks`. The frontend owns the chunking; the
/// store only keeps what it is given.
#[derive(Debug, Deserialize)]
pub struct ChunkUpsertInput {
    pub chunk_index: u32,
    pub chunk_text: String,
    pub heading_path: String,
    pub embedding: Vec<f32>,
}

const MAX_PAGE_ID_CHARS: usize = 256;

/// Page ids are wiki-relative paths without `.md`: Unicode letters, spaces,
/// slashes and punctuation are fine. Control, format and invisible characters
/// are rejected so visually identical ids cannot differ only by a soft hyphen,
/// zero-width or bidi character.
pub(crate) fn validate_page_id_for_v2(page_id: &str) -> Result<(), String> {
    if page_id.is_empty() {
        return Err("Invalid page_id: empty or too long".to_string());
    }
    for (count, c) in page_id.chars().enumerate() {
        if count + 1 > MAX_PAGE_ID_CHARS {
            return Err("Invalid page_id: empty or too long".to_string());
        }
        if is_disallowed_page_id_char(c) {
            return Err(format!(
                "Invalid page_id: contains disallowed character {:?}: {}",
                c, page_id
            ));
        }
    }
    Ok(())
}

fn is_disallowed_page_id_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\\' | '\''
                | '"'
                | '\u{00AD}'
                | '\u{061C}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2028}'..='\u{202E}'
                | '\u{2060}'..='\u{206F}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{E0000}'..='\u{E007F}'
        )
}

/// The vector store and the Project a command names.
fn target(
    project_path: &str,
) -> Result<(&'static dyn llm_wiki_storage::ChunkVectors, llm_wiki_storage::ProjectId), String> {
    let platform = crate::platform::require()?;
    let vectors = platform
        .vectors
        .as_deref()
        .ok_or_else(|| "semantic search is not configured on this server".to_string())?;
    let (project, root) = platform
        .locator
        .resolve(project_path)
        .map_err(|error| error.to_string())?;
    if !root.is_root() {
        return Err(format!("not a project path: {project_path}"));
    }
    Ok((vectors, project))
}

/// A page id is its wiki path without the folder and `.md`.
fn page_rel(page_id: &str) -> Result<llm_wiki_storage::RelPath, String> {
    validate_page_id_for_v2(page_id)?;
    llm_wiki_storage::RelPath::parse(&format!("wiki/{page_id}.md")).map_err(|e| e.to_string())
}

fn page_id_of(path: &str) -> String {
    let id = path.strip_prefix("wiki/").unwrap_or(path);
    id.strip_suffix(".md").unwrap_or(id).to_string()
}

/// Replaces a page's chunks. An empty list changes nothing, so a failed run
/// cannot wipe embeddings that were fine.
pub async fn vector_upsert_chunks(
    project_path: String,
    page_id: String,
    chunks: Vec<ChunkUpsertInput>,
) -> Result<(), String> {
    run_guarded_async("vector_upsert_chunks", async move {
        let (vectors, project) = target(&project_path)?;
        let page = page_rel(&page_id)?;
        let chunks = chunks
            .into_iter()
            .map(|c| llm_wiki_storage::ChunkInput {
                chunk_index: c.chunk_index,
                chunk_text: c.chunk_text,
                heading_path: c.heading_path,
                embedding: c.embedding,
            })
            .collect();
        vectors
            .upsert_chunks(project, &page, chunks, None)
            .await
            .map_err(|e| e.to_string())
    })
    .await
}

/// Top-K chunk search. The frontend groups the chunks by page.
pub async fn vector_search_chunks(
    project_path: String,
    query_embedding: Vec<f32>,
    top_k: usize,
) -> Result<Vec<ChunkSearchResult>, String> {
    run_guarded_async("vector_search_chunks", async move {
        let (vectors, project) = target(&project_path)?;
        let hits = vectors
            .search_chunks(project, &query_embedding, top_k)
            .await
            .map_err(|e| e.to_string())?;
        Ok(hits
            .into_iter()
            .map(|hit| {
                let page_id = page_id_of(&hit.path);
                ChunkSearchResult {
                    chunk_id: format!("{page_id}#{}", hit.chunk_index),
                    page_id,
                    chunk_index: hit.chunk_index,
                    chunk_text: hit.chunk_text,
                    heading_path: hit.heading_path,
                    score: 1.0 / (1.0 + hit.distance),
                }
            })
            .collect())
    })
    .await
}

/// Deletes every chunk of a page.
pub async fn vector_delete_page(project_path: String, page_id: String) -> Result<(), String> {
    run_guarded_async("vector_delete_page", async move {
        let (vectors, project) = target(&project_path)?;
        vectors
            .delete_page(project, &page_rel(&page_id)?)
            .await
            .map_err(|e| e.to_string())
    })
    .await
}

/// Total chunks of the Project (not pages), for "the index has N chunks".
pub async fn vector_count_chunks(project_path: String) -> Result<usize, String> {
    run_guarded_async("vector_count_chunks", async move {
        let (vectors, project) = target(&project_path)?;
        vectors.count_chunks(project).await.map_err(|e| e.to_string())
    })
    .await
}

/// Drops every chunk of the Project, so a re-index reflects the wiki exactly.
pub async fn vector_clear_chunks(project_path: String) -> Result<(), String> {
    run_guarded_async("vector_clear_chunks", async move {
        let (vectors, project) = target(&project_path)?;
        vectors.clear(project).await.map_err(|e| e.to_string())
    })
    .await
}

/// pgvector keeps its own index; there is nothing to compact.
pub async fn vector_optimize_chunks(project_path: String) -> Result<(), String> {
    target(&project_path).map(|_| ())
}

/// A platform Project never had the LanceDB table.
pub async fn vector_legacy_row_count(project_path: String) -> Result<usize, String> {
    target(&project_path).map(|_| 0)
}

pub async fn vector_drop_legacy(project_path: String) -> Result<(), String> {
    target(&project_path).map(|_| ())
}
