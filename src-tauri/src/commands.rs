//! Tauri command wrappers over `llm_wiki_core`.
//!
//! Keep each wrapper a one-line pass-through: behavior belongs in the core
//! crate so desktop IPC and server RPC (plans/web-server-mode.md) stay in
//! lockstep. Argument names and sync/async are part of the frontend
//! `invoke` contract — a sync command runs on the main thread. To add a
//! command, write it in `llm_wiki_core`, add a wrapper here, and register
//! it in `lib.rs`.

pub mod fs {
    use llm_wiki_core as core;
    use llm_wiki_core::types::wiki::FileNode;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::fs::*;

    #[tauri::command]
    pub async fn read_file(path: String, extract_images: Option<bool>) -> Result<String, String> {
        core::commands::fs::read_file(path, extract_images).await
    }

    #[tauri::command]
    pub async fn write_file(path: String, contents: String) -> Result<(), String> {
        core::commands::fs::write_file(path, contents).await
    }

    #[tauri::command]
    pub async fn write_file_base64(path: String, base64: String) -> Result<(), String> {
        core::commands::fs::write_file_base64(path, base64).await
    }

    #[tauri::command]
    pub async fn write_file_atomic(path: String, contents: String) -> Result<(), String> {
        core::commands::fs::write_file_atomic(path, contents).await
    }

    #[tauri::command]
    pub async fn apply_text_selection_edit(
        project_path: String,
        file_path: String,
        prefix: String,
        selected_text: String,
        suffix: String,
        replacement: String,
    ) -> Result<String, String> {
        core::commands::fs::apply_text_selection_edit(
            project_path,
            file_path,
            prefix,
            selected_text,
            suffix,
            replacement,
        )
        .await
    }

    #[tauri::command]
    pub async fn create_missing_wiki_page(
        project_path: String,
        title: String,
        content: Option<String>,
    ) -> Result<String, String> {
        core::commands::fs::create_missing_wiki_page(project_path, title, content).await
    }

    #[tauri::command]
    pub async fn list_directory(
        path: String,
        include_hidden: Option<bool>,
        max_depth: Option<usize>,
    ) -> Result<Vec<FileNode>, String> {
        core::commands::fs::list_directory(path, include_hidden, max_depth).await
    }

    #[tauri::command]
    pub async fn copy_file(source: String, destination: String) -> Result<(), String> {
        core::commands::fs::copy_file(source, destination).await
    }

    #[tauri::command]
    pub async fn copy_directory(
        source: String,
        destination: String,
    ) -> Result<Vec<String>, String> {
        core::commands::fs::copy_directory(source, destination).await
    }

    #[tauri::command]
    pub async fn preprocess_file(path: String) -> Result<String, String> {
        core::commands::fs::preprocess_file(path).await
    }

    #[tauri::command]
    pub async fn delete_file(path: String) -> Result<(), String> {
        core::commands::fs::delete_file(path).await
    }

    #[tauri::command]
    pub async fn find_related_wiki_pages(
        project_path: String,
        source_name: String,
    ) -> Result<Vec<String>, String> {
        core::commands::fs::find_related_wiki_pages(project_path, source_name).await
    }

    #[tauri::command]
    pub async fn create_directory(path: String) -> Result<(), String> {
        core::commands::fs::create_directory(path).await
    }

    #[tauri::command]
    pub async fn file_exists(path: String) -> Result<bool, String> {
        core::commands::fs::file_exists(path).await
    }

    #[tauri::command]
    pub async fn get_file_modified_time(path: String) -> Result<u64, String> {
        core::commands::fs::get_file_modified_time(path).await
    }

    #[tauri::command]
    pub async fn get_file_size(path: String) -> Result<u64, String> {
        core::commands::fs::get_file_size(path).await
    }

    #[tauri::command]
    pub async fn get_file_md5(path: String) -> Result<String, String> {
        core::commands::fs::get_file_md5(path).await
    }

    #[tauri::command]
    pub async fn read_file_as_base64(path: String) -> Result<FileBase64, String> {
        core::commands::fs::read_file_as_base64(path).await
    }
}

pub mod file_history {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::file_history::*;

    #[tauri::command]
    pub async fn list_file_history(
        project_path: String,
        file_path: String,
    ) -> Result<Vec<FileHistoryEntry>, String> {
        core::commands::file_history::list_file_history(project_path, file_path).await
    }

    #[tauri::command]
    pub async fn restore_file_history(
        project_path: String,
        file_path: String,
        entry_id: String,
    ) -> Result<String, String> {
        core::commands::file_history::restore_file_history(project_path, file_path, entry_id).await
    }

    #[tauri::command]
    pub async fn get_file_history_stats(project_path: String) -> Result<FileHistoryStats, String> {
        core::commands::file_history::get_file_history_stats(project_path).await
    }

    #[tauri::command]
    pub async fn get_file_history_settings(
        project_path: String,
    ) -> Result<FileHistorySettings, String> {
        core::commands::file_history::get_file_history_settings(project_path).await
    }

    #[tauri::command]
    pub async fn set_file_history_settings(
        project_path: String,
        settings: FileHistorySettings,
    ) -> Result<FileHistorySettings, String> {
        core::commands::file_history::set_file_history_settings(project_path, settings).await
    }

    #[tauri::command]
    pub async fn clear_file_history(project_path: String) -> Result<(), String> {
        core::commands::file_history::clear_file_history(project_path).await
    }
}

pub mod project {
    use llm_wiki_core as core;
    use llm_wiki_core::types::wiki::WikiProject;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::project::*;

    #[tauri::command]
    pub fn create_project(name: String, path: String) -> Result<WikiProject, String> {
        core::commands::project::create_project(name, path)
    }

    #[tauri::command]
    pub fn open_project(path: String) -> Result<WikiProject, String> {
        core::commands::project::open_project(path)
    }
}

pub mod project_maintenance {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::project_maintenance::*;

    #[tauri::command]
    pub async fn export_project_archive(
        project_path: String,
        destination: String,
    ) -> Result<(), String> {
        core::commands::project_maintenance::export_project_archive(project_path, destination).await
    }

    #[tauri::command]
    pub async fn import_project_archive(
        archive_path: String,
        destination: String,
    ) -> Result<String, String> {
        core::commands::project_maintenance::import_project_archive(archive_path, destination).await
    }

    #[tauri::command]
    pub async fn rebuild_wiki_index(project_path: String) -> Result<RebuildIndexResult, String> {
        core::commands::project_maintenance::rebuild_wiki_index(project_path).await
    }
}

pub mod search {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::search::*;

    #[tauri::command]
    pub async fn search_project(
        project_path: String,
        query: String,
        top_k: Option<usize>,
        include_content: Option<bool>,
        query_embedding: Option<Vec<f32>>,
        embedding_config: Option<SearchEmbeddingConfig>,
    ) -> Result<ProjectSearchResponse, String> {
        core::commands::search::search_project(
            project_path,
            query,
            top_k,
            include_content,
            query_embedding,
            embedding_config,
        )
        .await
    }

    #[tauri::command]
    pub async fn embedding_fetch(
        text: String,
        cfg: SearchEmbeddingConfig,
        max_retries: Option<usize>,
    ) -> Result<Vec<f32>, String> {
        core::commands::search::embedding_fetch(text, cfg, max_retries).await
    }

    #[tauri::command]
    pub async fn embedding_fetch_batch(
        texts: Vec<String>,
        cfg: SearchEmbeddingConfig,
    ) -> Result<Vec<Vec<f32>>, String> {
        core::commands::search::embedding_fetch_batch(texts, cfg).await
    }

    #[tauri::command]
    pub async fn get_page_links(
        project_path: String,
        file_path: String,
    ) -> Result<PageLinksResponse, String> {
        core::commands::search::get_page_links(project_path, file_path).await
    }
}

pub mod external_search {
    use llm_wiki_core as core;
    use llm_wiki_core::agent::tools::{AnyTxtConfig, WebSearchConfig};

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::external_search::*;

    #[tauri::command]
    pub async fn web_search(
        query: String,
        config: WebSearchConfig,
        max_results: Option<usize>,
    ) -> Result<Vec<ExternalSearchResult>, String> {
        core::commands::external_search::web_search(query, config, max_results).await
    }

    #[tauri::command]
    pub async fn anytxt_search(
        query: String,
        config: AnyTxtConfig,
        max_results: Option<usize>,
    ) -> Result<Vec<ExternalSearchResult>, String> {
        core::commands::external_search::anytxt_search(query, config, max_results).await
    }
}

pub mod app {
    use llm_wiki_core as core;
    use llm_wiki_core::proxy;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::app::*;

    #[tauri::command]
    pub fn clip_server_status() -> String {
        core::commands::app::clip_server_status()
    }

    #[tauri::command]
    pub fn api_server_status() -> String {
        core::commands::app::api_server_status()
    }

    #[tauri::command]
    pub fn api_server_reload_config() -> String {
        core::commands::app::api_server_reload_config()
    }

    #[tauri::command]
    pub fn set_proxy_env(config: proxy::ProxyConfig) -> String {
        core::commands::app::set_proxy_env(config)
    }
}

pub mod agent_commands {
    use llm_wiki_core as core;
    use llm_wiki_core::CoreContext;
    use std::sync::Arc;
    use tauri::State;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::agent::commands::*;

    #[tauri::command]
    pub async fn agent_start_turn(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        request: core::agent::AgentChatRequest,
        llm_config: Option<core::agent::provider::LlmConfig>,
    ) -> Result<core::agent::types::AgentChatResponse, String> {
        core::agent::commands::agent_start_turn(ctx.inner(), project_id, request, llm_config).await
    }

    #[tauri::command]
    pub async fn agent_start_turn_stream(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        request: core::agent::AgentChatRequest,
        llm_config: Option<core::agent::provider::LlmConfig>,
    ) -> Result<String, String> {
        core::agent::commands::agent_start_turn_stream(
            Arc::clone(ctx.inner()),
            project_id,
            request,
            llm_config,
        )
        .await
    }

    #[tauri::command]
    pub fn agent_cancel_turn(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        session_id: String,
        run_id: Option<String>,
    ) -> Result<bool, String> {
        core::agent::commands::agent_cancel_turn(ctx.inner(), project_id, session_id, run_id)
    }

    #[tauri::command]
    pub fn agent_get_session(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        session_id: String,
        limit: Option<usize>,
    ) -> Result<Vec<core::agent::session::AgentSessionMessage>, String> {
        core::agent::commands::agent_get_session(ctx.inner(), project_id, session_id, limit)
    }

    #[tauri::command]
    pub fn agent_list_sessions(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
    ) -> Result<Vec<core::agent::session::AgentSession>, String> {
        core::agent::commands::agent_list_sessions(ctx.inner(), project_id)
    }
}

pub mod agent_skills {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::agent::skills::*;

    #[tauri::command]
    pub fn agent_list_skills(project_path: String) -> Vec<AvailableAgentSkill> {
        core::agent::skills::agent_list_skills(project_path)
    }
}

pub mod vectorstore {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::vectorstore::*;

    #[tauri::command]
    pub async fn vector_upsert(
        project_path: String,
        page_id: String,
        embedding: Vec<f32>,
    ) -> Result<(), String> {
        core::commands::vectorstore::vector_upsert(project_path, page_id, embedding).await
    }

    #[tauri::command]
    pub async fn vector_search(
        project_path: String,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>, String> {
        core::commands::vectorstore::vector_search(project_path, query_embedding, top_k).await
    }

    #[tauri::command]
    pub async fn vector_delete(project_path: String, page_id: String) -> Result<(), String> {
        core::commands::vectorstore::vector_delete(project_path, page_id).await
    }

    #[tauri::command]
    pub async fn vector_count(project_path: String) -> Result<usize, String> {
        core::commands::vectorstore::vector_count(project_path).await
    }

    #[tauri::command]
    pub async fn vector_upsert_chunks(
        project_path: String,
        page_id: String,
        chunks: Vec<ChunkUpsertInput>,
    ) -> Result<(), String> {
        core::commands::vectorstore::vector_upsert_chunks(project_path, page_id, chunks).await
    }

    #[tauri::command]
    pub async fn vector_search_chunks(
        project_path: String,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<ChunkSearchResult>, String> {
        core::commands::vectorstore::vector_search_chunks(project_path, query_embedding, top_k)
            .await
    }

    #[tauri::command]
    pub async fn vector_delete_page(project_path: String, page_id: String) -> Result<(), String> {
        core::commands::vectorstore::vector_delete_page(project_path, page_id).await
    }

    #[tauri::command]
    pub async fn vector_count_chunks(project_path: String) -> Result<usize, String> {
        core::commands::vectorstore::vector_count_chunks(project_path).await
    }

    #[tauri::command]
    pub async fn vector_clear_chunks(project_path: String) -> Result<(), String> {
        core::commands::vectorstore::vector_clear_chunks(project_path).await
    }

    #[tauri::command]
    pub async fn vector_optimize_chunks(project_path: String) -> Result<(), String> {
        core::commands::vectorstore::vector_optimize_chunks(project_path).await
    }

    #[tauri::command]
    pub async fn vector_legacy_row_count(project_path: String) -> Result<usize, String> {
        core::commands::vectorstore::vector_legacy_row_count(project_path).await
    }

    #[tauri::command]
    pub async fn vector_drop_legacy(project_path: String) -> Result<(), String> {
        core::commands::vectorstore::vector_drop_legacy(project_path).await
    }
}

pub mod claude_cli {
    use llm_wiki_core as core;
    use llm_wiki_core::CoreContext;
    use std::sync::Arc;
    use tauri::State;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::claude_cli::*;

    #[tauri::command]
    pub async fn claude_cli_detect() -> Result<DetectResult, String> {
        core::commands::claude_cli::claude_cli_detect().await
    }

    #[tauri::command]
    pub async fn claude_cli_spawn(
        ctx: State<'_, Arc<CoreContext>>,
        stream_id: String,
        model: String,
        messages: Vec<ClaudeMessage>,
        isolate_local_config: bool,
        working_directory: Option<String>,
    ) -> Result<(), String> {
        core::commands::claude_cli::claude_cli_spawn(
            ctx.events().clone(),
            &ctx.claude_cli,
            stream_id,
            model,
            messages,
            isolate_local_config,
            working_directory,
        )
        .await
    }

    #[tauri::command]
    pub async fn claude_cli_kill(
        ctx: State<'_, Arc<CoreContext>>,
        stream_id: String,
    ) -> Result<(), String> {
        core::commands::claude_cli::claude_cli_kill(&ctx.claude_cli, stream_id).await
    }
}

pub mod codex_cli {
    use llm_wiki_core as core;
    use llm_wiki_core::CoreContext;
    use std::sync::Arc;
    use tauri::State;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::codex_cli::*;

    #[tauri::command]
    pub async fn codex_cli_detect() -> Result<DetectResult, String> {
        core::commands::codex_cli::codex_cli_detect().await
    }

    #[tauri::command]
    pub async fn codex_cli_spawn(
        ctx: State<'_, Arc<CoreContext>>,
        stream_id: String,
        model: String,
        prompt: String,
        isolate_local_config: bool,
        timeout_minutes: Option<u64>,
        working_directory: Option<String>,
    ) -> Result<(), String> {
        core::commands::codex_cli::codex_cli_spawn(
            ctx.events().clone(),
            &ctx.codex_cli,
            stream_id,
            model,
            prompt,
            isolate_local_config,
            timeout_minutes,
            working_directory,
        )
        .await
    }

    #[tauri::command]
    pub async fn codex_cli_kill(
        ctx: State<'_, Arc<CoreContext>>,
        stream_id: String,
    ) -> Result<(), String> {
        core::commands::codex_cli::codex_cli_kill(&ctx.codex_cli, stream_id).await
    }
}

pub mod extract_images {
    use llm_wiki_core as core;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::extract_images::*;

    #[tauri::command]
    pub async fn extract_pdf_images_cmd(path: String) -> Result<Vec<ExtractedImage>, String> {
        core::commands::extract_images::extract_pdf_images_cmd(path).await
    }

    #[tauri::command]
    pub async fn extract_office_images_cmd(path: String) -> Result<Vec<ExtractedImage>, String> {
        core::commands::extract_images::extract_office_images_cmd(path).await
    }

    #[tauri::command]
    pub async fn extract_and_save_pdf_images_cmd(
        source_path: String,
        dest_dir: String,
        rel_to: String,
    ) -> Result<Vec<SavedImage>, String> {
        core::commands::extract_images::extract_and_save_pdf_images_cmd(
            source_path,
            dest_dir,
            rel_to,
        )
        .await
    }

    #[tauri::command]
    pub async fn extract_and_save_office_images_cmd(
        source_path: String,
        dest_dir: String,
        rel_to: String,
    ) -> Result<Vec<SavedImage>, String> {
        core::commands::extract_images::extract_and_save_office_images_cmd(
            source_path,
            dest_dir,
            rel_to,
        )
        .await
    }
}

pub mod file_sync {
    use llm_wiki_core as core;
    use llm_wiki_core::CoreContext;
    use std::sync::Arc;
    use tauri::State;

    // Brings the core module's argument/return types into scope.
    #[allow(unused_imports)]
    use llm_wiki_core::commands::file_sync::*;

    #[tauri::command]
    pub fn start_project_file_watcher(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        project_path: String,
        source_watch_config: Option<SourceWatchConfig>,
    ) -> Result<FileChangeRescanResult, String> {
        core::commands::file_sync::start_project_file_watcher(
            ctx.events().clone(),
            &ctx.file_sync,
            project_id,
            project_path,
            source_watch_config,
        )
    }

    #[tauri::command]
    pub fn stop_project_file_watcher(ctx: State<'_, Arc<CoreContext>>) -> Result<(), String> {
        core::commands::file_sync::stop_project_file_watcher(&ctx.file_sync)
    }

    #[tauri::command]
    pub fn rescan_project_files(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        project_path: String,
        source_watch_config: Option<SourceWatchConfig>,
        watch_roots_only: Option<bool>,
    ) -> Result<FileChangeRescanResult, String> {
        core::commands::file_sync::rescan_project_files(
            ctx.events().clone(),
            project_id,
            project_path,
            source_watch_config,
            watch_roots_only,
        )
    }

    #[tauri::command]
    pub fn invalidate_project_file_snapshot_paths(
        project_path: String,
        paths: Vec<String>,
    ) -> Result<(), String> {
        core::commands::file_sync::invalidate_project_file_snapshot_paths(project_path, paths)
    }

    #[tauri::command]
    pub fn get_file_change_queue(project_path: String) -> Result<FileChangeQueue, String> {
        core::commands::file_sync::get_file_change_queue(project_path)
    }

    #[tauri::command]
    pub fn retry_file_change_task(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        project_path: String,
        task_id: String,
    ) -> Result<FileChangeQueue, String> {
        core::commands::file_sync::retry_file_change_task(
            ctx.events().clone(),
            project_id,
            project_path,
            task_id,
        )
    }

    #[tauri::command]
    pub fn ignore_file_change_task(
        ctx: State<'_, Arc<CoreContext>>,
        project_id: String,
        project_path: String,
        task_id: String,
    ) -> Result<FileChangeQueue, String> {
        core::commands::file_sync::ignore_file_change_task(
            ctx.events().clone(),
            project_id,
            project_path,
            task_id,
        )
    }
}
