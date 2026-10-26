-- Project content behind the fs seam (ADR-0002).
--   text under `wiki/` plus the root `purpose.md` / `schema.md`  -> pages
--   everything binary (`raw/**`, `wiki/media/**`)                -> blobs (MinIO)
-- Paths keep the folder-era semantics: relative to the Project root, forward
-- slashes. Directories are not rows; listings derive them from path prefixes.

CREATE TABLE pages (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id     uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    path           text NOT NULL CHECK (path !~ '(^/|\.\./|//)'),
    -- Current state, projected from the newest `page_versions` row so reads
    -- never join.
    content        text NOT NULL,
    content_sha256 bytea NOT NULL,
    frontmatter    jsonb NOT NULL DEFAULT '{}',
    title          text,
    page_type      text,
    -- Optimistic lock: a writer states the version it read; the UPDATE only
    -- matches if it is still current.
    version        integer NOT NULL DEFAULT 1,
    -- Soft delete so a Changeset can be reverted. Paths are reusable once deleted.
    deleted_at     timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX pages_live_path_key ON pages (project_id, path) WHERE deleted_at IS NULL;
CREATE INDEX pages_project_type_idx ON pages (project_id, page_type) WHERE deleted_at IS NULL;
-- Serves `path LIKE 'dir/%'` (directory listings, subtree delete and rename).
CREATE INDEX pages_live_prefix_idx ON pages (project_id, path text_pattern_ops) WHERE deleted_at IS NULL;

-- Append-only; one row per write, including the first and including deletes.
-- Replaces both `.llm-wiki/page-history/` (backup before ingest/dedup
-- overwrite) and `.llm-wiki/history/` (file_history.rs).
CREATE TABLE page_versions (
    id             bigserial PRIMARY KEY,
    page_id        uuid NOT NULL REFERENCES pages (id) ON DELETE CASCADE,
    version        integer NOT NULL,
    op             text NOT NULL CHECK (op IN ('create', 'update', 'delete', 'rename', 'restore')),
    -- NULL content on 'delete'.
    content        text,
    content_sha256 bytea,
    frontmatter    jsonb,
    -- Which write path produced it: 'edit', 'ingest', 'dedup', 'agent', 'restore', ...
    tool           text NOT NULL,
    author_id      uuid REFERENCES users (id) ON DELETE SET NULL,
    -- Groups the pages one ingest run wrote; becomes the Changeset key in phase 3.
    ingest_job_id  uuid,
    created_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (page_id, version)
);
CREATE INDEX page_versions_job_idx ON page_versions (ingest_job_id) WHERE ingest_job_id IS NOT NULL;

-- Edges for the graph, backlinks and lint. Rebuilt for a page inside the same
-- transaction that writes it.
CREATE TABLE page_links (
    project_id     uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    source_page_id uuid NOT NULL REFERENCES pages (id) ON DELETE CASCADE,
    -- The `[[target]]` text, normalised the way the resolver compares it.
    target_ref     text NOT NULL,
    -- NULL = broken link (what lint reports). Re-resolved when a page with a
    -- matching ref appears or disappears.
    target_page_id uuid REFERENCES pages (id) ON DELETE SET NULL,
    PRIMARY KEY (source_page_id, target_ref)
);
CREATE INDEX page_links_target_idx ON page_links (target_page_id) WHERE target_page_id IS NOT NULL;
CREATE INDEX page_links_ref_idx ON page_links (project_id, target_ref);

-- Binary objects in MinIO. The object key is `<project_id>/<id>`, so renaming
-- or moving a file is a row update and deleting a Project is one prefix delete.
-- Uploads and downloads stream through the API; no presigned URLs.
-- Objects are immutable: the company MinIO allows at most 10 versions per key,
-- so a changed file is a new row with a new id (new key), never a re-PUT.
CREATE TABLE blobs (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id   uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    path         text NOT NULL CHECK (path !~ '(^/|\.\./|//)'),
    -- 'source' = user upload (a Source), 'extracted' = parsed markdown of a
    -- Source, 'asset' = image extracted from a Source or under wiki/media,
    -- 'workspace' = artifact the agent generated under agent-workspace/.
    kind         text NOT NULL CHECK (kind IN ('source', 'extracted', 'asset', 'workspace')),
    size_bytes   bigint NOT NULL,
    sha256       bytea NOT NULL,
    mime_type    text,
    uploaded_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    deleted_at   timestamptz
);
CREATE UNIQUE INDEX blobs_live_path_key ON blobs (project_id, path) WHERE deleted_at IS NULL;
CREATE INDEX blobs_project_kind_idx ON blobs (project_id, kind) WHERE deleted_at IS NULL;
CREATE INDEX blobs_sha_idx ON blobs (project_id, sha256);

-- Derived Page -> Source provenance (the `sources:` frontmatter list), so
-- "which pages depend on this Source" is a query, not a scan.
CREATE TABLE page_sources (
    page_id     uuid NOT NULL REFERENCES pages (id) ON DELETE CASCADE,
    project_id  uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    source_ref  text NOT NULL,
    blob_id     uuid REFERENCES blobs (id) ON DELETE SET NULL,
    PRIMARY KEY (page_id, source_ref)
);
CREATE INDEX page_sources_blob_idx ON page_sources (blob_id) WHERE blob_id IS NOT NULL;
CREATE INDEX page_sources_ref_idx ON page_sources (project_id, source_ref);
