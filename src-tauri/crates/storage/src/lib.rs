//! Project file storage (ADR-0002).
//!
//! Everything that used to read a Project folder with `std::fs` goes through
//! [`ProjectFiles`]. Paths keep their folder-era meaning: relative to the
//! Project root, forward slashes. Where a path is stored (pages in Postgres,
//! blobs in MinIO, `.llm-wiki/*` state in tables) is decided by [`route`], not
//! by callers.

mod error;
mod listing;
mod locator;
mod memory;
mod path;
mod pg;
mod route;
mod store;

#[cfg(test)]
mod conformance;

pub use error::StoreError;
pub use locator::ProjectLocator;
pub use memory::MemoryStore;
pub use path::RelPath;
pub use pg::{migrate, PgFiles, DEFAULT_MAX_VERSIONS};
pub use route::{route, Backend};
pub use store::{Body, EntryKind, Meta, Precondition, ProjectFiles, ProjectId, WriteCtx};
