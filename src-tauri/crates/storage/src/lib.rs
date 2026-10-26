//! Project file storage (ADR-0002).
//!
//! Everything that used to read a Project folder with `std::fs` goes through
//! [`ProjectFiles`]. Paths keep their folder-era meaning: relative to the
//! Project root, forward slashes. Where a path is stored (pages in Postgres,
//! blobs in MinIO, `.llm-wiki/*` state in tables) is decided by [`route`], not
//! by callers.

mod accounts;
mod blobs;
mod connect;
mod derive;
mod index;
mod error;
mod listing;
mod locator;
mod memory;
mod objects;
mod path;
mod pg;
mod projects;
mod route;
mod routed;
mod settings;
mod sql;
mod state;
mod store;

#[cfg(test)]
mod conformance;

pub use accounts::{check_password, normalize_username, Accounts, AuthError, NewUser, User};
pub use blobs::PgBlobs;
pub use connect::{open_services, open_store, Services};
pub use error::StoreError;
pub use locator::ProjectLocator;
pub use memory::MemoryStore;
pub use objects::S3Config;
pub use path::RelPath;
pub use index::{LinkTarget, PageLinks, PageRef};
pub use pg::{migrate, PgFiles, DEFAULT_MAX_VERSIONS};
pub use projects::{Member, ProjectInfo, Projects, Role};
pub use route::{blob_kind, route, Backend};
pub use routed::RoutedFiles;
pub use settings::{LlmUsage, ServerSettings, UsageRecord, UsageTotal, LLM_MODELS_KEY};
pub use state::{PgState, MAX_STATE_BYTES};
pub use store::{
    BlobStreams, Body, ByteStream, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx,
};
