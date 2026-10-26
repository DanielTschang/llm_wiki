//! Project file storage (ADR-0002).
//!
//! Everything that used to read a Project folder with `std::fs` goes through
//! [`ProjectFiles`]. Paths keep their folder-era meaning: relative to the
//! Project root, forward slashes. Where a path is stored (pages in Postgres,
//! blobs in MinIO, `.llm-wiki/*` state in tables) is decided by [`route`], not
//! by callers.

mod blobs;
mod connect;
mod error;
mod listing;
mod locator;
mod memory;
mod objects;
mod path;
mod pg;
mod route;
mod routed;
mod sql;
mod store;

#[cfg(test)]
mod conformance;

pub use blobs::PgBlobs;
pub use connect::open_store;
pub use error::StoreError;
pub use locator::ProjectLocator;
pub use memory::MemoryStore;
pub use objects::S3Config;
pub use path::RelPath;
pub use pg::{migrate, PgFiles, DEFAULT_MAX_VERSIONS};
pub use route::{blob_kind, route, Backend};
pub use routed::RoutedFiles;
pub use store::{
    BlobStreams, Body, ByteStream, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx,
};
