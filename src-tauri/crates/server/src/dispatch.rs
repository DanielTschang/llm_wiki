//! RPC dispatch table: one arm per backend command (argument names are the
//! camelCase keys the frontend passes to `invoke`). Sync commands run on the
//! blocking pool.

use std::sync::Arc;

use llm_wiki_core as core;
use llm_wiki_core::CoreContext;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum DispatchError {
    Unknown,
    Failed(String),
    /// Refused with a specific status, e.g. 403 when the role is too low.
    Status(axum::http::StatusCode, String),
}

pub async fn dispatch(
    ctx: &Arc<CoreContext>,
    command: &str,
    args: Value,
) -> Result<Value, DispatchError> {
    match command {
        "read_file" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
                extract_images: Option<bool>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::read_file(a.path, a.extract_images).await)
        }
        "write_file" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
                contents: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::write_file(a.path, a.contents).await)
        }
        "write_file_base64" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
                base64: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::write_file_base64(a.path, a.base64).await)
        }
        "write_file_atomic" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
                contents: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::write_file_atomic(a.path, a.contents).await)
        }
        "apply_text_selection_edit" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                file_path: String,
                prefix: String,
                selected_text: String,
                suffix: String,
                replacement: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::fs::apply_text_selection_edit(
                    a.project_path,
                    a.file_path,
                    a.prefix,
                    a.selected_text,
                    a.suffix,
                    a.replacement,
                )
                .await,
            )
        }
        "create_missing_wiki_page" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                title: String,
                content: Option<String>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::fs::create_missing_wiki_page(a.project_path, a.title, a.content)
                    .await,
            )
        }
        "list_directory" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
                include_hidden: Option<bool>,
                max_depth: Option<usize>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::fs::list_directory(a.path, a.include_hidden, a.max_depth).await,
            )
        }
        "copy_file" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                source: String,
                destination: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::copy_file(a.source, a.destination).await)
        }
        "copy_directory" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                source: String,
                destination: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::copy_directory(a.source, a.destination).await)
        }
        "preprocess_file" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::preprocess_file(a.path).await)
        }
        "delete_file" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::delete_file(a.path).await)
        }
        "find_related_wiki_pages" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                source_name: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::fs::find_related_wiki_pages(a.project_path, a.source_name).await,
            )
        }
        "create_directory" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::create_directory(a.path).await)
        }
        "file_exists" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::file_exists(a.path).await)
        }
        "get_file_modified_time" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::get_file_modified_time(a.path).await)
        }
        "get_file_size" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::get_file_size(a.path).await)
        }
        "get_file_md5" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::get_file_md5(a.path).await)
        }
        "read_file_as_base64" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::fs::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::FileNode;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::fs::read_file_as_base64(a.path).await)
        }
        "search_project" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                query: String,
                top_k: Option<usize>,
                include_content: Option<bool>,
                query_embedding: Option<Vec<f32>>,
                embedding_config: Option<SearchEmbeddingConfig>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::search::search_project(
                    a.project_path,
                    a.query,
                    a.top_k,
                    a.include_content,
                    a.query_embedding,
                    a.embedding_config,
                )
                .await,
            )
        }
        "embedding_fetch" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                text: String,
                cfg: SearchEmbeddingConfig,
                max_retries: Option<usize>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::search::embedding_fetch(a.text, a.cfg, a.max_retries).await)
        }
        "embedding_fetch_batch" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                texts: Vec<String>,
                cfg: SearchEmbeddingConfig,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::search::embedding_fetch_batch(a.texts, a.cfg).await)
        }
        "get_page_links" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                file_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::search::get_page_links(a.project_path, a.file_path).await)
        }
        "web_search" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::tools::{AnyTxtConfig, WebSearchConfig};
            #[allow(unused_imports)]
            use llm_wiki_core::commands::external_search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                query: String,
                config: WebSearchConfig,
                max_results: Option<usize>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::external_search::web_search(a.query, a.config, a.max_results).await,
            )
        }
        "anytxt_search" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::tools::{AnyTxtConfig, WebSearchConfig};
            #[allow(unused_imports)]
            use llm_wiki_core::commands::external_search::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                query: String,
                config: AnyTxtConfig,
                max_results: Option<usize>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::external_search::anytxt_search(a.query, a.config, a.max_results)
                    .await,
            )
        }
        "set_proxy_env" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::app::*;
            #[allow(unused_imports)]
            use llm_wiki_core::proxy;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                config: proxy::ProxyConfig,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_value(rpc_blocking(move || core::commands::app::set_proxy_env(a.config)).await?)
        }
        "agent_start_turn" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::commands::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                request: core::agent::AgentChatRequest,
                llm_config: Option<core::agent::provider::LlmConfig>,
                embedding_config: Option<llm_wiki_core::commands::search::SearchEmbeddingConfig>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                core::agent::commands::agent_start_turn(
                    &ctx,
                    a.project_id,
                    a.request,
                    a.llm_config,
                    a.embedding_config,
                )
                .await,
            )
        }
        "agent_start_turn_stream" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::commands::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                request: core::agent::AgentChatRequest,
                llm_config: Option<core::agent::provider::LlmConfig>,
                embedding_config: Option<llm_wiki_core::commands::search::SearchEmbeddingConfig>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                core::agent::commands::agent_start_turn_stream(
                    Arc::clone(&ctx),
                    a.project_id,
                    a.request,
                    a.llm_config,
                    a.embedding_config,
                )
                .await,
            )
        }
        "agent_cancel_turn" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::commands::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                session_id: String,
                run_id: Option<String>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::agent::commands::agent_cancel_turn(
                        &ctx,
                        a.project_id,
                        a.session_id,
                        a.run_id,
                    )
                })
                .await?,
            )
        }
        "agent_get_session" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::commands::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                session_id: String,
                limit: Option<usize>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::agent::commands::agent_get_session(
                        &ctx,
                        a.project_id,
                        a.session_id,
                        a.limit,
                    )
                })
                .await?,
            )
        }
        "agent_list_sessions" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::commands::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::agent::commands::agent_list_sessions(&ctx, a.project_id)
                })
                .await?,
            )
        }
        "agent_list_skills" => {
            #[allow(unused_imports)]
            use llm_wiki_core::agent::skills::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_value(
                rpc_blocking(move || core::agent::skills::agent_list_skills(a.project_path))
                    .await?,
            )
        }
        "vector_upsert_chunks" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                page_id: String,
                chunks: Vec<ChunkUpsertInput>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::vectorstore::vector_upsert_chunks(
                    a.project_path,
                    a.page_id,
                    a.chunks,
                )
                .await,
            )
        }
        "vector_search_chunks" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                query_embedding: Vec<f32>,
                top_k: usize,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::vectorstore::vector_search_chunks(
                    a.project_path,
                    a.query_embedding,
                    a.top_k,
                )
                .await,
            )
        }
        "vector_delete_page" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                page_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::vectorstore::vector_delete_page(a.project_path, a.page_id).await,
            )
        }
        "vector_count_chunks" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_count_chunks(a.project_path).await)
        }
        "vector_clear_chunks" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_clear_chunks(a.project_path).await)
        }
        "vector_optimize_chunks" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_optimize_chunks(a.project_path).await)
        }
        "vector_legacy_row_count" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_legacy_row_count(a.project_path).await)
        }
        "vector_drop_legacy" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_drop_legacy(a.project_path).await)
        }
        "extract_pdf_images_cmd" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::extract_images::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::extract_images::extract_pdf_images_cmd(a.path).await)
        }
        "extract_office_images_cmd" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::extract_images::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::extract_images::extract_office_images_cmd(a.path).await)
        }
        "extract_and_save_pdf_images_cmd" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::extract_images::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                source_path: String,
                dest_dir: String,
                rel_to: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::extract_images::extract_and_save_pdf_images_cmd(
                    a.source_path,
                    a.dest_dir,
                    a.rel_to,
                )
                .await,
            )
        }
        "extract_and_save_office_images_cmd" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::extract_images::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                source_path: String,
                dest_dir: String,
                rel_to: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::extract_images::extract_and_save_office_images_cmd(
                    a.source_path,
                    a.dest_dir,
                    a.rel_to,
                )
                .await,
            )
        }
        _ => Err(DispatchError::Unknown),
    }
}

fn rpc_args<T: DeserializeOwned>(args: Value) -> Result<T, DispatchError> {
    serde_json::from_value(args)
        .map_err(|e| DispatchError::Failed(format!("Invalid arguments: {e}")))
}

fn rpc_reply<T: Serialize>(result: Result<T, String>) -> Result<Value, DispatchError> {
    result.map_err(DispatchError::Failed).and_then(rpc_value)
}

fn rpc_value<T: Serialize>(value: T) -> Result<Value, DispatchError> {
    serde_json::to_value(value).map_err(|e| DispatchError::Failed(e.to_string()))
}

async fn rpc_blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, DispatchError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| DispatchError::Failed(format!("Command task failed: {e}")))
}
