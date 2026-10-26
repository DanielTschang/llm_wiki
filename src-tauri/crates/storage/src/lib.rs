//! Project file storage (ADR-0002).
//!
//! Everything that used to read a Project folder with `std::fs` goes through
//! [`ProjectFiles`]. Paths keep their folder-era meaning: relative to the
//! Project root, forward slashes. Where a path is stored (pages in Postgres,
//! blobs in MinIO, `.llm-wiki/*` state in tables) is decided by [`route`], not
//! by callers.

mod blobs;
mod error;
mod listing;
mod locator;
mod memory;
mod path;
mod pg;
mod route;
mod sql;
mod store;

#[cfg(test)]
mod conformance;

pub use blobs::PgBlobs;
pub use error::StoreError;
pub use locator::ProjectLocator;
pub use memory::MemoryStore;
pub use path::RelPath;
pub use pg::{migrate, PgFiles, DEFAULT_MAX_VERSIONS};
pub use route::{blob_kind, route, Backend};
pub use store::{
    BlobStreams, Body, ByteStream, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx,
};
