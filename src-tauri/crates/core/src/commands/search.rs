use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use llm_wiki_storage::RelPath;

use crate::commands::vectorstore;
use crate::panic_guard::run_guarded_async;

const DEFAULT_RESULTS: usize = 20;
const MAX_RESULTS: usize = 50;
const RRF_K: f64 = 60.0;
const FILENAME_EXACT_BONUS: f64 = 200.0;
const PHRASE_IN_TITLE_BONUS: f64 = 50.0;
const PHRASE_IN_CONTENT_PER_OCC: f64 = 20.0;
const MAX_PHRASE_OCC_COUNTED: usize = 10;
const TITLE_TOKEN_WEIGHT: f64 = 5.0;
const CONTENT_TOKEN_WEIGHT: f64 = 1.0;
const SNIPPET_CONTEXT: usize = 80;
use llm_wiki_llm::SearchEmbeddingConfig;
use llm_wiki_llm::embedding::{
    fetch_embedding_batch, fetch_embedding_with_retry,
};
const MAX_SEARCH_FILES: usize = 10_000;
const MIN_GRAPH_RESULT_RATIO: f64 = 0.15;
const MAX_GRAPH_RESULT_RATIO: f64 = 0.30;
const MAX_GRAPH_SEEDS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchImageRef {
    pub url: String,
    pub alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub title_match: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_score: Option<f32>,
    pub images: Vec<SearchImageRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graph_related_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSearchResponse {
    pub mode: String,
    pub results: Vec<ProjectSearchResult>,
    pub token_hits: usize,
    pub vector_hits: usize,
    pub graph_hits: usize,
}

#[derive(Debug, Clone)]
struct GraphPage {
    path: String,
    title: String,
    content: String,
    links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageLinkEntry {
    pub title: String,
    pub path: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageLinksResponse {
    pub outgoing: Vec<PageLinkEntry>,
    pub backlinks: Vec<PageLinkEntry>,
    pub missing: Vec<PageLinkEntry>,
}


pub async fn search_project(
    project_path: String,
    query: String,
    top_k: Option<usize>,
    include_content: Option<bool>,
    query_embedding: Option<Vec<f32>>,
    embedding_config: Option<SearchEmbeddingConfig>,
) -> Result<ProjectSearchResponse, String> {
    run_guarded_async("search_project", async move {
        let query_embedding =
            resolve_query_embedding(&query, query_embedding, embedding_config).await?;
        search_project_inner(
            project_path,
            query,
            top_k.unwrap_or(DEFAULT_RESULTS),
            include_content.unwrap_or(false),
            query_embedding,
        )
        .await
    })
    .await
}

pub async fn embedding_fetch(
    text: String,
    cfg: SearchEmbeddingConfig,
    max_retries: Option<usize>,
) -> Result<Vec<f32>, String> {
    run_guarded_async("embedding_fetch", async move {
        fetch_embedding_with_retry(&text, &cfg, max_retries.unwrap_or(3)).await
    })
    .await
}

pub async fn embedding_fetch_batch(
    texts: Vec<String>,
    cfg: SearchEmbeddingConfig,
) -> Result<Vec<Vec<f32>>, String> {
    run_guarded_async("embedding_fetch_batch", async move {
        fetch_embedding_batch(&texts, &cfg).await
    })
    .await
}

pub async fn get_page_links(
    project_path: String,
    file_path: String,
) -> Result<PageLinksResponse, String> {
    let platform = crate::platform::require()?;
    get_page_links_from_store(platform, &project_path, &file_path).await
}

/// `get_page_links` for a Project held in the storage layer.
async fn get_page_links_from_store(
    platform: &crate::platform::Platform,
    project_path: &str,
    file_path: &str,
) -> Result<PageLinksResponse, String> {
    let (project, _) = platform
        .locator
        .resolve(project_path)
        .map_err(|err| err.to_string())?;
    let (file_project, file) = platform
        .locator
        .resolve(file_path)
        .map_err(|err| err.to_string())?;
    let is_markdown = file
        .extension()
        .is_some_and(|ext| ext == "md");
    if file_project != project || file.top_level() != "wiki" || !is_markdown {
        return Err("Page links target must be an existing Markdown file under wiki/".to_string());
    }
    let (body, _) = platform
        .files
        .read(project, &file)
        .await
        .map_err(|_| "Page is not available in the current wiki index".to_string())?;
    let file_name = file.as_str().rsplit('/').next().unwrap_or_default();
    let current_title = extract_title(body.as_text().unwrap_or_default(), file_name);
    // The link tables are kept up to date by every page write, so this is
    // two queries instead of a scan of the wiki.
    let links = platform
        .files
        .page_links(project, &file)
        .await
        .map_err(|err| err.to_string())?;

    let mut outgoing = Vec::new();
    let mut missing = Vec::new();
    for link in links.outgoing {
        match link.path {
            Some(path) => outgoing.push(PageLinkEntry {
                title: link.title.unwrap_or_else(|| link.target_ref.clone()),
                path: Some(path),
                snippet: None,
            }),
            None => missing.push(PageLinkEntry {
                title: link.target_ref,
                path: None,
                snippet: None,
            }),
        }
    }
    let mut backlinks = Vec::new();
    for page in links.backlinks {
        let snippet = match RelPath::parse(&page.path) {
            Ok(rel) => platform.files.read(project, &rel).await.ok().and_then(|(body, _)| {
                body.as_text().map(|text| build_snippet(text, &current_title))
            }),
            Err(_) => None,
        };
        backlinks.push(PageLinkEntry {
            title: page.title.unwrap_or_else(|| page.path.clone()),
            path: Some(page.path),
            snippet,
        });
    }
    outgoing.sort_by(|a, b| a.title.cmp(&b.title));
    outgoing.dedup_by(|a, b| a.path == b.path);
    backlinks.sort_by(|a, b| a.title.cmp(&b.title));
    missing.sort_by(|a, b| a.title.cmp(&b.title));
    missing.dedup_by(|a, b| a.title == b.title);
    Ok(PageLinksResponse {
        outgoing,
        backlinks,
        missing,
    })
}




/// Every Markdown page of the wiki as (path, content), at most
/// `MAX_SEARCH_FILES`. On a platform server the paths are Project paths under
/// `/projects/<uuid>`; otherwise they are real files under `project_path`.
async fn load_wiki_pages(project_path: &str) -> Result<Vec<(PathBuf, String)>, String> {
    let platform = crate::platform::require()?;
    let root = Path::new(project_path);
    Ok(platform
        .wiki_markdown(project_path, MAX_SEARCH_FILES)
        .await?
        .into_iter()
        .map(|(rel, content)| (root.join(rel), content))
        .collect())
}

pub async fn resolve_query_embedding(
    query: &str,
    explicit_embedding: Option<Vec<f32>>,
    embedding_config: Option<SearchEmbeddingConfig>,
) -> Result<Option<Vec<f32>>, String> {
    if let Some(embedding) = explicit_embedding {
        return validate_query_embedding(embedding).map(Some);
    }
    let Some(cfg) = embedding_config else {
        return Ok(None);
    };
    if !cfg.enabled || cfg.endpoint.trim().is_empty() || cfg.model.trim().is_empty() {
        return Ok(None);
    }
    match fetch_embedding_with_retry(query, &cfg, 0).await {
        Ok(embedding) => validate_query_embedding(embedding).map(Some),
        Err(err) => {
            eprintln!("[Search] embedding disabled for this request: {err}");
            Ok(None)
        }
    }
}

fn validate_query_embedding(embedding: Vec<f32>) -> Result<Vec<f32>, String> {
    if embedding.is_empty() {
        return Err("queryEmbedding must not be empty".to_string());
    }
    if embedding.iter().any(|v| !v.is_finite()) {
        return Err("queryEmbedding must contain only finite numbers".to_string());
    }
    Ok(embedding)
}

pub async fn search_project_inner(
    project_path: String,
    query: String,
    top_k: usize,
    include_content: bool,
    query_embedding: Option<Vec<f32>>,
) -> Result<ProjectSearchResponse, String> {
    if query.trim().is_empty() {
        return Err("query is required".to_string());
    }
    let limit = top_k.clamp(1, MAX_RESULTS);
    let tokens = tokenize_query(&query);
    let effective_tokens = if tokens.is_empty() {
        vec![query.trim().to_lowercase()]
    } else {
        tokens
    };
    let query_phrase = trim_query_punctuation(&query.to_lowercase());
    let mut results = Vec::new();
    let mut page_paths_by_vector_id = BTreeMap::new();
    let mut legacy_stem_candidates = BTreeMap::<String, Vec<String>>::new();
    let mut graph_pages = BTreeMap::new();

    for (path, content) in load_wiki_pages(&project_path).await? {
        let relative_path = relative_to_project(&project_path, &path);
        if let Some(vector_id) = vector_page_id(&relative_path) {
            page_paths_by_vector_id.insert(vector_id, relative_path.clone());
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            legacy_stem_candidates
                .entry(stem.to_string())
                .or_default()
                .push(relative_path.clone());
        }
        let title = extract_title(
            &content,
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default(),
        );
        let hit = score_file(
            &project_path,
            &path,
            &content,
            &effective_tokens,
            &query_phrase,
            &query,
            include_content,
        );
        graph_pages.insert(
            normalize_path(&relative_path),
            GraphPage {
                path: relative_path,
                title,
                links: extract_graph_links(&content),
                content,
            },
        );
        if let Some(hit) = hit {
            results.push(hit);
        }
    }
    let legacy_page_paths = legacy_stem_candidates
        .into_iter()
        .filter_map(|(stem, paths)| (paths.len() == 1).then(|| (stem, paths[0].clone())))
        .collect::<BTreeMap<_, _>>();

    let mut token_sorted = (0..results.len()).collect::<Vec<_>>();
    token_sorted.sort_by(|a, b| {
        results[*b]
            .score
            .partial_cmp(&results[*a].score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| results[*a].path.cmp(&results[*b].path))
    });
    let mut token_rank = BTreeMap::new();
    for (idx, result_idx) in token_sorted.iter().enumerate() {
        let result = &results[*result_idx];
        token_rank.insert(normalize_path(&result.path), idx + 1);
    }

    let mut vector_rank: BTreeMap<String, usize> = BTreeMap::new();
    let mut vector_score: BTreeMap<String, f32> = BTreeMap::new();
    let mut vector_hits = 0;
    if let Some(embedding) = query_embedding {
        if !embedding.is_empty() {
            match search_by_embedding(&project_path, embedding, limit.max(10)).await {
                Ok(vector_results) => {
                    vector_hits = vector_results.len();
                    for (idx, vr) in vector_results.iter().enumerate() {
                        vector_rank.insert(vr.id.clone(), idx + 1);
                        vector_score.insert(vr.id.clone(), vr.score);
                    }
                    materialize_with(
                        &vector_results,
                        &page_paths_by_vector_id,
                        &legacy_page_paths,
                        &mut results,
                        include_content,
                        |rel| {
                            graph_pages
                                .get(&normalize_path(rel))
                                .map(|page: &GraphPage| page.content.clone())
                        },
                    );
                }
                Err(err) => {
                    eprintln!(
                        "[Search] vector search failed; falling back to keyword results: {err}"
                    );
                }
            }
        }
    }

    if vector_hits > 0 {
        apply_rrf_scores(
            &mut results,
            &token_rank,
            &vector_rank,
            &vector_score,
            &legacy_page_paths,
        );
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    let graph_hits = blend_graph_results(
        &mut results,
        &graph_pages,
        limit,
        vector_hits,
        include_content,
    );

    Ok(ProjectSearchResponse {
        mode: search_mode(token_rank.is_empty(), vector_hits, graph_hits).to_string(),
        token_hits: token_rank.len(),
        vector_hits,
        graph_hits,
        results,
    })
}

fn apply_rrf_scores(
    results: &mut [ProjectSearchResult],
    token_rank: &BTreeMap<String, usize>,
    vector_rank: &BTreeMap<String, usize>,
    vector_score: &BTreeMap<String, f32>,
    legacy_page_paths: &BTreeMap<String, String>,
) {
    for result in results {
        let token = token_rank.get(&normalize_path(&result.path)).copied();
        let vector_id = vector_page_id(&result.path).unwrap_or_else(|| file_stem(&result.path));
        let legacy_stem = file_stem(&result.path);
        let legacy_matches_page = legacy_page_paths
            .get(&legacy_stem)
            .is_some_and(|path| normalize_path(path) == normalize_path(&result.path));
        let vector = vector_rank
            .get(&vector_id)
            .or_else(|| {
                legacy_matches_page
                    .then(|| vector_rank.get(&legacy_stem))
                    .flatten()
            })
            .copied();
        let mut rrf = 0.0;
        if let Some(rank) = token {
            rrf += 1.0 / (RRF_K + rank as f64);
        }
        if let Some(rank) = vector {
            rrf += 1.0 / (RRF_K + rank as f64);
        }
        if let Some(score) = vector_score
            .get(&vector_id)
            .or_else(|| {
                legacy_matches_page
                    .then(|| vector_score.get(&legacy_stem))
                    .flatten()
            })
            .copied()
        {
            result.vector_score = Some(score);
        }
        result.score = rrf;
    }
}

/// Reserve 15-30% of the final window for one-hop graph expansion. A full
/// vector window leaves the minimum graph share; sparse vector retrieval moves
/// progressively toward the maximum. Missing graph candidates automatically
/// return their slots to keyword/vector results.
fn graph_result_quota(limit: usize, vector_hits: usize) -> usize {
    if limit < 2 {
        return 0;
    }
    let vector_coverage = vector_hits.min(limit) as f64 / limit as f64;
    let ratio = MAX_GRAPH_RESULT_RATIO
        - (MAX_GRAPH_RESULT_RATIO - MIN_GRAPH_RESULT_RATIO) * vector_coverage;
    ((limit as f64 * ratio).ceil() as usize).clamp(1, limit - 1)
}

fn blend_graph_results(
    ranked_results: &mut Vec<ProjectSearchResult>,
    pages: &BTreeMap<String, GraphPage>,
    limit: usize,
    vector_hits: usize,
    include_content: bool,
) -> usize {
    if ranked_results.is_empty() || pages.is_empty() {
        ranked_results.truncate(limit);
        return 0;
    }

    let mut aliases = BTreeMap::<String, String>::new();
    for (normalized_path, page) in pages {
        let wiki_relative = page.path.strip_prefix("wiki/").unwrap_or(&page.path);
        let stem = file_stem(&page.path);
        for alias in [
            page.path.as_str(),
            wiki_relative,
            stem.as_str(),
            page.title.as_str(),
        ] {
            aliases.insert(normalize_graph_alias(alias), normalized_path.clone());
        }
    }

    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for (source, page) in pages {
        for link in &page.links {
            let Some(target) = aliases.get(&normalize_graph_alias(link)) else {
                continue;
            };
            if source == target {
                continue;
            }
            adjacency
                .entry(source.clone())
                .or_default()
                .insert(target.clone());
            adjacency
                .entry(target.clone())
                .or_default()
                .insert(source.clone());
        }
    }

    let seed_paths: Vec<String> = ranked_results
        .iter()
        .take(limit.min(MAX_GRAPH_SEEDS))
        .map(|result| normalize_path(&result.path))
        .collect();
    let seed_set: BTreeSet<String> = seed_paths.iter().cloned().collect();
    let mut candidate_scores = BTreeMap::<String, f64>::new();
    let mut candidate_seeds = BTreeMap::<String, BTreeSet<String>>::new();
    for (rank, seed) in seed_paths.iter().enumerate() {
        let Some(neighbors) = adjacency.get(seed) else {
            continue;
        };
        for neighbor in neighbors {
            if seed_set.contains(neighbor) {
                continue;
            }
            *candidate_scores.entry(neighbor.clone()).or_default() += 1.0 / (rank + 1) as f64;
            if let Some(seed_page) = pages.get(seed) {
                candidate_seeds
                    .entry(neighbor.clone())
                    .or_default()
                    .insert(seed_page.title.clone());
            }
        }
    }

    let mut candidates: Vec<(String, f64)> = candidate_scores.into_iter().collect();
    candidates.sort_by(|(path_a, score_a), (path_b, score_b)| {
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| path_a.cmp(path_b))
    });
    candidates.truncate(graph_result_quota(limit, vector_hits));
    if candidates.is_empty() {
        ranked_results.truncate(limit);
        return 0;
    }

    let selected_paths: BTreeSet<String> =
        candidates.iter().map(|(path, _)| path.clone()).collect();
    let mut existing = BTreeMap::<String, ProjectSearchResult>::new();
    let mut ranked_paths = Vec::new();
    for result in ranked_results.drain(..) {
        let path = normalize_path(&result.path);
        ranked_paths.push(path.clone());
        existing.insert(path, result);
    }

    let graph_count = candidates.len();
    let base_limit = limit.saturating_sub(graph_count);
    let mut base_results: Vec<ProjectSearchResult> = ranked_paths
        .iter()
        .filter(|path| !selected_paths.contains(*path))
        .filter_map(|path| existing.get(path).cloned())
        .take(base_limit)
        .collect();

    for (path, graph_score) in candidates {
        if let Some(result) = existing.remove(&path) {
            let mut result = result;
            result.graph_related_to = candidate_seeds
                .remove(&path)
                .unwrap_or_default()
                .into_iter()
                .collect();
            base_results.push(result);
            continue;
        }
        let Some(page) = pages.get(&path) else {
            continue;
        };
        let related_titles = candidate_seeds
            .remove(&path)
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        let related = related_titles.join(", ");
        base_results.push(ProjectSearchResult {
            path: page.path.clone(),
            title: page.title.clone(),
            snippet: format!("Graph neighbor of {related}"),
            title_match: false,
            score: graph_score / (RRF_K + 1.0),
            vector_score: None,
            images: extract_image_refs(&page.content),
            content: include_content.then(|| page.content.clone()),
            graph_related_to: related_titles,
        });
    }
    *ranked_results = base_results;
    graph_count
}

fn normalize_graph_alias(value: &str) -> String {
    value
        .split('#')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches(".md")
        .replace('\\', "/")
        .replace(' ', "-")
        .to_lowercase()
}

pub(crate) fn extract_graph_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let target = rest[..end].split('|').next().unwrap_or_default().trim();
        if !target.is_empty() {
            links.push(target.to_string());
        }
        rest = &rest[end + 2..];
    }
    for related in extract_related_frontmatter(content) {
        if !links.contains(&related) {
            links.push(related);
        }
    }
    links
}

fn extract_related_frontmatter(content: &str) -> Vec<String> {
    let normalized = content.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Vec::new();
    };
    let Some(end) = rest.find("\n---") else {
        return Vec::new();
    };
    let lines: Vec<&str> = rest[..end].lines().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let Some(value) = line.trim().strip_prefix("related:") else {
            index += 1;
            continue;
        };
        let value = value.trim();
        if value.starts_with('[') && value.ends_with(']') {
            for item in value[1..value.len() - 1].split(',') {
                if let Some(item) = normalize_related_value(item) {
                    out.push(item);
                }
            }
        } else if value.is_empty() {
            index += 1;
            while index < lines.len() {
                let trimmed = lines[index].trim();
                let Some(item) = trimmed.strip_prefix('-') else {
                    break;
                };
                if let Some(item) = normalize_related_value(item) {
                    out.push(item);
                }
                index += 1;
            }
            continue;
        }
        index += 1;
    }
    out
}

fn normalize_related_value(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"').trim_matches('\'').trim();
    let value = value
        .strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))
        .unwrap_or(value)
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn search_mode(token_rank_empty: bool, vector_hits: usize, graph_hits: usize) -> &'static str {
    if graph_hits > 0 {
        "hybrid"
    } else if vector_hits == 0 {
        "keyword"
    } else if token_rank_empty {
        "vector"
    } else {
        "hybrid"
    }
}

#[derive(Debug, Clone)]
struct PageVectorResult {
    id: String,
    score: f32,
    chunk_text: String,
    heading_path: String,
}

async fn search_by_embedding(
    project_path: &str,
    query_embedding: Vec<f32>,
    top_k: usize,
) -> Result<Vec<PageVectorResult>, String> {
    let raw_chunks = vectorstore::vector_search_chunks(
        project_path.to_string(),
        query_embedding,
        (top_k * 3).max(30),
    )
    .await?;
    if raw_chunks.is_empty() {
        return Ok(vec![]);
    }

    let mut by_page: BTreeMap<String, Vec<vectorstore::ChunkSearchResult>> = BTreeMap::new();
    for chunk in raw_chunks {
        by_page
            .entry(chunk.page_id.clone())
            .or_default()
            .push(chunk);
    }

    let mut ranked = Vec::new();
    for (id, mut chunks) in by_page {
        chunks.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk_index.cmp(&b.chunk_index))
        });
        let top_chunk = chunks[0].clone();
        let top = top_chunk.score;
        let tail: f32 = chunks.iter().skip(1).map(|chunk| chunk.score).sum();
        let blended = top + (tail * 0.3).min((1.0 - top).max(0.0));
        ranked.push(PageVectorResult {
            id,
            score: blended,
            chunk_text: top_chunk.chunk_text,
            heading_path: top_chunk.heading_path,
        });
    }
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    ranked.truncate(top_k);
    Ok(ranked)
}


/// Adds a result for each vector hit whose page the keyword scan did not
/// already return. `read` gives a page's text from its project-relative path.
fn materialize_with(
    vector_results: &[PageVectorResult],
    page_paths_by_vector_id: &BTreeMap<String, String>,
    legacy_page_paths: &BTreeMap<String, String>,
    results: &mut Vec<ProjectSearchResult>,
    include_content: bool,
    read: impl Fn(&str) -> Option<String>,
) {
    let mut known_paths: BTreeSet<String> = results
        .iter()
        .map(|result| normalize_path(&result.path))
        .collect();
    // Materialize path-qualified rows first. During migration a page can have
    // both a current row and a legacy basename row; the current score/snippet
    // must win regardless of the backend's result order.
    for qualified_only in [true, false] {
        for vr in vector_results {
            let rel = if qualified_only {
                page_paths_by_vector_id.get(&vr.id)
            } else if page_paths_by_vector_id.contains_key(&vr.id) {
                None
            } else {
                legacy_page_paths.get(&vr.id)
            };
            let Some(rel) = rel else {
                continue;
            };
            let normalized_rel = normalize_path(rel);
            if known_paths.contains(&normalized_rel) {
                continue;
            }
            let Some(content) = read(rel) else {
                continue;
            };
            let file_name = Path::new(&rel)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let title = extract_title(&content, file_name);
            let snippet = build_vector_snippet(vr);
            results.push(ProjectSearchResult {
                path: rel.clone(),
                title,
                snippet,
                title_match: false,
                score: 0.0,
                vector_score: Some(vr.score),
                images: extract_image_refs(&content),
                content: include_content.then_some(content),
                graph_related_to: Vec::new(),
            });
            known_paths.insert(normalized_rel);
        }
    }
}

fn build_vector_snippet(result: &PageVectorResult) -> String {
    let mut text = result.chunk_text.trim().replace('\n', " ");
    if text.is_empty() {
        return String::new();
    }
    if text.chars().count() > SNIPPET_CONTEXT * 2 {
        text = text.chars().take(SNIPPET_CONTEXT * 2).collect::<String>();
        text.push_str("...");
    }
    let heading = result.heading_path.trim();
    if heading.is_empty() {
        text
    } else {
        format!("{heading}: {text}")
    }
}

fn score_file(
    project_path: &str,
    path: &Path,
    content: &str,
    tokens: &[String],
    query_phrase: &str,
    query: &str,
    include_content: bool,
) -> Option<ProjectSearchResult> {
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let title = extract_title(content, file_name);
    let title_text = format!("{title} {file_name}");
    let title_lower = title_text.to_lowercase();
    let content_lower = content.to_lowercase();
    let stem = file_name.trim_end_matches(".md").to_lowercase();

    let filename_exact = !query_phrase.is_empty() && stem == query_phrase;
    let title_has_phrase = !query_phrase.is_empty() && title_lower.contains(query_phrase);
    let content_phrase_occ =
        count_occurrences(&content_lower, query_phrase).min(MAX_PHRASE_OCC_COUNTED);
    let title_token_score = token_match_score(&title_text, tokens);
    let content_token_score = token_match_score(content, tokens);

    if !filename_exact
        && !title_has_phrase
        && content_phrase_occ == 0
        && title_token_score == 0
        && content_token_score == 0
    {
        return None;
    }

    let score = (if filename_exact {
        FILENAME_EXACT_BONUS
    } else {
        0.0
    }) + (if title_has_phrase {
        PHRASE_IN_TITLE_BONUS
    } else {
        0.0
    }) + content_phrase_occ as f64 * PHRASE_IN_CONTENT_PER_OCC
        + title_token_score as f64 * TITLE_TOKEN_WEIGHT
        + content_token_score as f64 * CONTENT_TOKEN_WEIGHT;

    let snippet_anchor = if content_phrase_occ > 0 {
        query_phrase.to_string()
    } else {
        tokens
            .iter()
            .find(|token| content_lower.contains(token.as_str()))
            .cloned()
            .unwrap_or_else(|| query.to_string())
    };

    Some(ProjectSearchResult {
        path: relative_to_project(project_path, path),
        title,
        snippet: build_snippet(content, &snippet_anchor),
        title_match: title_token_score > 0 || title_has_phrase,
        score,
        vector_score: None,
        images: extract_image_refs(content),
        content: include_content.then_some(content.to_string()),
        graph_related_to: Vec::new(),
    })
}

pub fn tokenize_query(query: &str) -> Vec<String> {
    let raw = query
        .to_lowercase()
        .split(is_query_separator)
        .filter(|token| token.chars().count() > 1)
        .filter(|token| !is_stop_word(token))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let mut out = Vec::new();
    for token in raw {
        let chars = token.chars().collect::<Vec<_>>();
        let has_cjk = chars.iter().any(|c| ('\u{3400}'..='\u{9fff}').contains(c));
        if has_cjk && chars.len() > 2 {
            for pair in chars.windows(2) {
                out.push(pair.iter().collect());
            }
            for ch in &chars {
                let s = ch.to_string();
                if !is_stop_word(&s) {
                    out.push(s);
                }
            }
            out.push(token);
        } else {
            out.push(token);
        }
    }
    out.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_query_separator(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || matches!(
            c,
            '，' | '。'
                | '！'
                | '？'
                | '、'
                | '；'
                | '：'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '（'
                | '）'
                | '·'
                | '～'
                | '…'
        )
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "的" | "是"
            | "了"
            | "什么"
            | "在"
            | "有"
            | "和"
            | "与"
            | "对"
            | "从"
            | "the"
            | "is"
            | "a"
            | "an"
            | "what"
            | "how"
            | "are"
            | "was"
            | "were"
            | "do"
            | "does"
            | "did"
            | "be"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "it"
            | "its"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "this"
            | "that"
            | "these"
            | "those"
    )
}

fn trim_query_punctuation(value: &str) -> String {
    value.trim_matches(is_query_separator).to_string()
}

fn token_match_score(text: &str, tokens: &[String]) -> usize {
    let lower = text.to_lowercase();
    tokens
        .iter()
        .filter(|token| lower.contains(token.as_str()))
        .count()
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

pub fn extract_title(content: &str, file_name: &str) -> String {
    let has_frontmatter = content.starts_with("---");
    let mut in_frontmatter = has_frontmatter;
    let mut frontmatter_closed = false;
    for line in content.lines().skip(if has_frontmatter { 1 } else { 0 }) {
        let trimmed = line.trim();
        if in_frontmatter && trimmed == "---" {
            in_frontmatter = false;
            frontmatter_closed = true;
            continue;
        }
        if in_frontmatter && trimmed.starts_with("title:") {
            return trimmed
                .trim_start_matches("title:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
        }
        if has_frontmatter && !frontmatter_closed {
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            return title.trim().to_string();
        }
    }
    file_name.trim_end_matches(".md").replace('-', " ")
}

pub fn extract_image_refs(content: &str) -> Vec<SearchImageRef> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut rest = content;
    while let Some(start) = rest.find("![") {
        rest = &rest[start + 2..];
        let Some(alt_end) = rest.find("](") else {
            break;
        };
        let alt = &rest[..alt_end];
        rest = &rest[alt_end + 2..];
        let Some(url_end) = rest.find(')') else {
            break;
        };
        let url = &rest[..url_end];
        if !url.trim().is_empty()
            && !url.contains(char::is_whitespace)
            && seen.insert(url.to_string())
        {
            out.push(SearchImageRef {
                url: url.to_string(),
                alt: alt.to_string(),
            });
        }
        rest = &rest[url_end + 1..];
    }
    out
}


pub fn build_snippet(content: &str, query: &str) -> String {
    let lower = content.to_lowercase();
    let q = query.to_lowercase();
    let idx = lower.find(&q).unwrap_or(0);
    let char_positions: Vec<usize> = content.char_indices().map(|(idx, _)| idx).collect();
    if char_positions.is_empty() {
        return String::new();
    }
    let match_char = char_positions
        .iter()
        .position(|byte| *byte >= idx)
        .unwrap_or(char_positions.len().saturating_sub(1));
    let query_chars = query.chars().count().max(1);
    let start_char = match_char.saturating_sub(SNIPPET_CONTEXT);
    let end_char = (match_char + query_chars + SNIPPET_CONTEXT).min(char_positions.len());
    let start = char_positions[start_char];
    let end = if end_char < char_positions.len() {
        char_positions[end_char]
    } else {
        content.len()
    };
    let mut snippet = content[start..end].replace('\n', " ");
    if start > 0 {
        snippet = format!("...{snippet}");
    }
    if end < content.len() {
        snippet.push_str("...");
    }
    snippet
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn relative_to_project(project_path: &str, path: &Path) -> String {
    let root = Path::new(project_path);
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

fn vector_page_id(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    let wiki_relative = normalized.strip_prefix("wiki/").unwrap_or(&normalized);
    if wiki_relative.len() < 3
        || !wiki_relative[wiki_relative.len() - 3..].eq_ignore_ascii_case(".md")
    {
        return None;
    }
    let id = wiki_relative[..wiki_relative.len() - 3].trim_matches('/');
    (!id.is_empty()).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    



    fn result(path: &str) -> ProjectSearchResult {
        ProjectSearchResult {
            path: path.to_string(),
            title: path.to_string(),
            snippet: String::new(),
            title_match: false,
            score: 0.0,
            vector_score: None,
            images: vec![],
            content: None,
            graph_related_to: Vec::new(),
        }
    }

    #[test]
    fn tokenizes_cjk_bigrams_and_chars() {
        let tokens = tokenize_query("默会知识");
        assert!(tokens.contains(&"默会".to_string()));
        assert!(tokens.contains(&"知识".to_string()));
        assert!(tokens.contains(&"默".to_string()));
    }

    #[test]
    fn extracts_image_refs_without_duplicates() {
        let refs = extract_image_refs("![a](wiki/media/x.png)\n![b](wiki/media/x.png)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].alt, "a");
    }

    #[test]
    fn graph_links_include_frontmatter_related_and_dedupe_body_links() {
        let links = extract_graph_links("---\nrelated: [foo, \"[[bar]]\"]\n---\n# Page\n[[foo]]");
        assert_eq!(links, vec!["bar", "foo"]);

        let block_links = extract_graph_links(
            "---\r\nrelated:\r\n  - wiki/entities/alpha.md\r\n  - beta\r\n---\r\n# Page",
        );
        assert_eq!(block_links, vec!["wiki/entities/alpha.md", "beta"]);
    }

    #[test]
    fn extract_title_uses_frontmatter_or_heading_not_body_title_lines() {
        let with_frontmatter = "---\ntitle: Real Title\n---\n\ntitle: Body Label\n# Heading";
        assert_eq!(
            extract_title(with_frontmatter, "fallback-name.md"),
            "Real Title"
        );

        let without_frontmatter = "intro\ntitle: Body Label\n# Real Heading";
        assert_eq!(
            extract_title(without_frontmatter, "fallback-name.md"),
            "Real Heading"
        );

        assert_eq!(
            extract_title("plain body", "vector-database.md"),
            "vector database"
        );
    }

    #[test]
    fn explicit_query_embedding_is_validated() {
        assert!(validate_query_embedding(vec![0.1, 0.2]).is_ok());
        assert!(validate_query_embedding(vec![]).is_err());
        assert!(validate_query_embedding(vec![f32::NAN]).is_err());
        assert!(validate_query_embedding(vec![f32::INFINITY]).is_err());
    }











    #[test]
    fn rrf_combines_token_and_vector_ranks_and_keeps_vector_score() {
        let mut results = vec![
            result("wiki/concepts/both.md"),
            result("wiki/concepts/token-only.md"),
            result("wiki/concepts/vector-only.md"),
        ];
        let token_rank = BTreeMap::from([
            ("wiki/concepts/both.md".to_string(), 1),
            ("wiki/concepts/token-only.md".to_string(), 2),
        ]);
        let vector_rank = BTreeMap::from([("both".to_string(), 1), ("vector-only".to_string(), 2)]);
        let vector_score =
            BTreeMap::from([("both".to_string(), 0.95), ("vector-only".to_string(), 0.8)]);

        let legacy_page_paths = BTreeMap::from([
            ("both".to_string(), "wiki/concepts/both.md".to_string()),
            (
                "vector-only".to_string(),
                "wiki/concepts/vector-only.md".to_string(),
            ),
        ]);
        apply_rrf_scores(
            &mut results,
            &token_rank,
            &vector_rank,
            &vector_score,
            &legacy_page_paths,
        );
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        assert_eq!(results[0].path, "wiki/concepts/both.md");
        assert!((results[0].score - (1.0 / 61.0 + 1.0 / 61.0)).abs() < 0.000001);
        assert_eq!(results[0].vector_score, Some(0.95));
        assert!((results[1].score - (1.0 / 62.0)).abs() < 0.000001);
        assert!((results[2].score - (1.0 / 62.0)).abs() < 0.000001);
    }

    #[test]
    fn search_mode_distinguishes_keyword_vector_and_hybrid() {
        assert_eq!(search_mode(false, 0, 0), "keyword");
        assert_eq!(search_mode(true, 3, 0), "vector");
        assert_eq!(search_mode(false, 3, 0), "hybrid");
        assert_eq!(search_mode(false, 0, 1), "hybrid");
    }

    #[test]
    fn graph_quota_scales_from_thirty_to_fifteen_percent() {
        assert_eq!(graph_result_quota(1, 0), 0);
        assert_eq!(graph_result_quota(20, 0), 6);
        assert_eq!(graph_result_quota(20, 10), 5);
        assert_eq!(graph_result_quota(20, 20), 3);
        assert_eq!(graph_result_quota(10, 100), 2);
    }



    #[test]
    fn vector_snippet_empty_chunk_does_not_echo_query() {
        let vector = PageVectorResult {
            id: "empty".to_string(),
            score: 0.5,
            chunk_text: "  ".to_string(),
            heading_path: "Heading".to_string(),
        };

        assert_eq!(build_vector_snippet(&vector), "");
    }








    mod on_a_platform_server {
        use std::sync::Arc;

        use llm_wiki_storage::{
            Body, MemoryStore, Precondition, ProjectFiles, ProjectId, ProjectLocator, RelPath,
            WriteCtx,
        };

        use super::*;
        use crate::platform::Platform;

        struct Wiki {
            platform: Platform,
            root: String,
        }

        async fn wiki(pages: &[(&str, &str)]) -> Wiki {
            let files = Arc::new(MemoryStore::new());
            let locator = ProjectLocator::default();
            let id = ProjectId(uuid::Uuid::new_v4());
            for (path, content) in pages {
                files
                    .write(
                        id,
                        &RelPath::parse(path).unwrap(),
                        Body::Text((*content).to_string()),
                        Precondition::Any,
                        &WriteCtx::new("test"),
                    )
                    .await
                    .unwrap();
            }
            Wiki {
                root: locator.project_path(id),
                platform: Platform {
                    vectors: None,
                    files,
                    streams: None,
                    locator,
                },
            }
        }

        #[tokio::test]
        async fn the_wiki_scan_returns_only_markdown_pages_under_wiki() {
            let w = wiki(&[
                ("wiki/b.md", "# B"),
                ("wiki/a.md", "# A"),
                ("wiki/sub/c.MD", "# C"),
                ("wiki/notes.txt", "text"),
                ("raw/sources/s.md", "source"),
                ("purpose.md", "why"),
            ])
            .await;
            let pages = w.platform.wiki_markdown(&w.root, 100).await.unwrap();
            let paths: Vec<_> = pages.iter().map(|(path, _)| path.as_str()).collect();
            assert_eq!(paths, ["wiki/a.md", "wiki/b.md", "wiki/sub/c.MD"]);
            assert_eq!(pages[0].1, "# A");
            assert!(w
                .platform
                .wiki_markdown(&format!("{}/wiki", w.root), 100)
                .await
                .is_err());
        }

        #[tokio::test]
        async fn page_links_are_built_from_the_stored_pages() {
            let w = wiki(&[
                ("wiki/a.md", "# A\nsee [[b]]"),
                ("wiki/b.md", "# B\nback to [[a]]"),
                ("wiki/c.md", "# C\n[[a]] and [[missing]]"),
                ("raw/sources/x.md", "[[a]]"),
            ])
            .await;
            let page = |name: &str| format!("{}/wiki/{name}", w.root);

            let a = get_page_links_from_store(&w.platform, &w.root, &page("a.md"))
                .await
                .unwrap();
            assert_eq!(a.outgoing.iter().map(|l| l.title.as_str()).collect::<Vec<_>>(), ["B"]);
            assert_eq!(a.backlinks.iter().map(|l| l.title.as_str()).collect::<Vec<_>>(), ["B", "C"]);
            assert!(a.missing.is_empty());
            assert_eq!(a.outgoing[0].path.as_deref(), Some("wiki/b.md"));

            let c = get_page_links_from_store(&w.platform, &w.root, &page("c.md"))
                .await
                .unwrap();
            assert_eq!(c.missing.iter().map(|l| l.title.as_str()).collect::<Vec<_>>(), ["missing"]);
        }

        #[tokio::test]
        async fn page_links_refuse_targets_outside_the_wiki_or_the_project() {
            let w = wiki(&[("wiki/a.md", "# A"), ("raw/sources/x.md", "x")]).await;
            let other = ProjectLocator::default().project_path(ProjectId(uuid::Uuid::new_v4()));
            for target in [
                format!("{}/raw/sources/x.md", w.root),
                format!("{}/wiki/a.txt", w.root),
                format!("{other}/wiki/a.md"),
                "/etc/passwd".to_string(),
            ] {
                assert!(
                    get_page_links_from_store(&w.platform, &w.root, &target)
                        .await
                        .is_err(),
                    "{target}"
                );
            }
            let missing = format!("{}/wiki/none.md", w.root);
            assert!(get_page_links_from_store(&w.platform, &w.root, &missing)
                .await
                .is_err());
        }
    }
}
