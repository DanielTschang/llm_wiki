//! RPC dispatch table: one arm per backend command, mirroring the desktop
//! wrappers in `src-tauri/src/commands.rs` (argument names are the same
//! camelCase keys the frontend passes to `invoke`). Sync commands run on the
//! blocking pool, as Tauri would run them off the async executor.
//! `tests::dispatches_every_desktop_command` fails when the two lists drift.

use std::sync::Arc;

use llm_wiki_core as core;
use llm_wiki_core::CoreContext;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum DispatchError {
    Unknown,
    Failed(String),
}

/// Desktop commands the server deliberately does not offer.
pub const SERVER_DISABLED: &[&str] = &[
    "claude_cli_detect",
    "claude_cli_kill",
    "claude_cli_spawn",
    "codex_cli_detect",
    "codex_cli_kill",
    "codex_cli_spawn",
];

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
        "list_file_history" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                file_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::file_history::list_file_history(a.project_path, a.file_path).await,
            )
        }
        "restore_file_history" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                file_path: String,
                entry_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::file_history::restore_file_history(
                    a.project_path,
                    a.file_path,
                    a.entry_id,
                )
                .await,
            )
        }
        "get_file_history_stats" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::file_history::get_file_history_stats(a.project_path).await)
        }
        "get_file_history_settings" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::file_history::get_file_history_settings(a.project_path).await)
        }
        "set_file_history_settings" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                settings: FileHistorySettings,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::file_history::set_file_history_settings(a.project_path, a.settings)
                    .await,
            )
        }
        "clear_file_history" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_history::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::file_history::clear_file_history(a.project_path).await)
        }
        "create_project" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::project::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::WikiProject;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                name: String,
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                rpc_blocking(move || core::commands::project::create_project(a.name, a.path))
                    .await?,
            )
        }
        "open_project" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::project::*;
            #[allow(unused_imports)]
            use llm_wiki_core::types::wiki::WikiProject;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(rpc_blocking(move || core::commands::project::open_project(a.path)).await?)
        }
        "export_project_archive" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::project_maintenance::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                destination: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::project_maintenance::export_project_archive(
                    a.project_path,
                    a.destination,
                )
                .await,
            )
        }
        "import_project_archive" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::project_maintenance::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                archive_path: String,
                destination: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::project_maintenance::import_project_archive(
                    a.archive_path,
                    a.destination,
                )
                .await,
            )
        }
        "rebuild_wiki_index" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::project_maintenance::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::project_maintenance::rebuild_wiki_index(a.project_path).await)
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
        "clip_server_status" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::app::*;
            #[allow(unused_imports)]
            use llm_wiki_core::proxy;
            rpc_value(rpc_blocking(core::commands::app::clip_server_status).await?)
        }
        "api_server_status" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::app::*;
            #[allow(unused_imports)]
            use llm_wiki_core::proxy;
            rpc_value(rpc_blocking(core::commands::app::api_server_status).await?)
        }
        "api_server_reload_config" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::app::*;
            #[allow(unused_imports)]
            use llm_wiki_core::proxy;
            rpc_value(rpc_blocking(core::commands::app::api_server_reload_config).await?)
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
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                core::agent::commands::agent_start_turn(
                    &ctx,
                    a.project_id,
                    a.request,
                    a.llm_config,
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
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                core::agent::commands::agent_start_turn_stream(
                    Arc::clone(&ctx),
                    a.project_id,
                    a.request,
                    a.llm_config,
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
        "vector_upsert" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                page_id: String,
                embedding: Vec<f32>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                core::commands::vectorstore::vector_upsert(a.project_path, a.page_id, a.embedding)
                    .await,
            )
        }
        "vector_search" => {
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
                core::commands::vectorstore::vector_search(
                    a.project_path,
                    a.query_embedding,
                    a.top_k,
                )
                .await,
            )
        }
        "vector_delete" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                page_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_delete(a.project_path, a.page_id).await)
        }
        "vector_count" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::vectorstore::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(core::commands::vectorstore::vector_count(a.project_path).await)
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
        "start_project_file_watcher" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                project_path: String,
                source_watch_config: Option<SourceWatchConfig>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::start_project_file_watcher(
                        ctx.events().clone(),
                        &ctx.file_sync,
                        a.project_id,
                        a.project_path,
                        a.source_watch_config,
                    )
                })
                .await?,
            )
        }
        "stop_project_file_watcher" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::stop_project_file_watcher(&ctx.file_sync)
                })
                .await?,
            )
        }
        "rescan_project_files" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                project_path: String,
                source_watch_config: Option<SourceWatchConfig>,
                watch_roots_only: Option<bool>,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::rescan_project_files(
                        ctx.events().clone(),
                        a.project_id,
                        a.project_path,
                        a.source_watch_config,
                        a.watch_roots_only,
                    )
                })
                .await?,
            )
        }
        "invalidate_project_file_snapshot_paths" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
                paths: Vec<String>,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::invalidate_project_file_snapshot_paths(
                        a.project_path,
                        a.paths,
                    )
                })
                .await?,
            )
        }
        "get_file_change_queue" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_path: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::get_file_change_queue(a.project_path)
                })
                .await?,
            )
        }
        "retry_file_change_task" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                project_path: String,
                task_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::retry_file_change_task(
                        ctx.events().clone(),
                        a.project_id,
                        a.project_path,
                        a.task_id,
                    )
                })
                .await?,
            )
        }
        "ignore_file_change_task" => {
            #[allow(unused_imports)]
            use llm_wiki_core::commands::file_sync::*;
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct RpcArgs {
                project_id: String,
                project_path: String,
                task_id: String,
            }
            let a: RpcArgs = rpc_args(args)?;
            let ctx = Arc::clone(ctx);
            rpc_reply(
                rpc_blocking(move || {
                    core::commands::file_sync::ignore_file_change_task(
                        ctx.events().clone(),
                        a.project_id,
                        a.project_path,
                        a.task_id,
                    )
                })
                .await?,
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

#[cfg(test)]
mod tests {
    use super::*;

    const DESKTOP_ONLY: &[&str] = &[
        "open_project_folder",
        "open_path_in_project",
        "mcp_server_entry_path",
        "set_close_behavior",
    ];

    fn desktop_commands() -> Vec<String> {
        let lib = include_str!("../../../src/lib.rs");
        let start =
            lib.find("generate_handler![").expect("handler list") + "generate_handler![".len();
        let end = start + lib[start..].find("])").expect("handler list end");
        lib[start..end]
            .split(',')
            .map(|entry| entry.trim().rsplit("::").next().unwrap_or("").to_string())
            .filter(|name| !name.is_empty())
            .collect()
    }

    #[tokio::test]
    async fn dispatches_every_desktop_command() {
        let ctx = Arc::new(CoreContext::new(None, "test", core::EventEmitter::noop()));
        let mut missing = Vec::new();
        for name in desktop_commands() {
            if DESKTOP_ONLY.contains(&name.as_str()) || SERVER_DISABLED.contains(&name.as_str()) {
                continue;
            }
            // Deliberately malformed arguments: a known command fails
            // argument parsing (or runs a harmless no-arg status call);
            // only an unknown one reports Unknown.
            if let Err(DispatchError::Unknown) =
                dispatch(&ctx, &name, Value::String("x".into())).await
            {
                missing.push(name);
            }
        }
        assert!(missing.is_empty(), "server does not dispatch: {missing:?}");
    }
}
