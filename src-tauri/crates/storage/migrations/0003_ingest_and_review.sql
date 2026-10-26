-- Ingest, dedup, review, lint, chat: the `.llm-wiki/*.json` state files that
-- become real tables (plans/inventory/llm-wiki-state-files.md).
-- Ids that the browser or worker already generate (review items, conversations,
-- messages) stay `text` and are scoped by project_id, so the folder-era id
-- semantics survive the fs seam unchanged.

-- Replaces ingest-queue.json, ingest-progress/*.json and ingest-generation/*.json.
-- Dispatch is by JetStream; this row is the durable state (see `outbox`).
CREATE TABLE ingest_jobs (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id     uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    source_path    text NOT NULL,
    folder_context text NOT NULL DEFAULT '',
    status         text NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'processing', 'done', 'failed', 'cancelled')),
    attempts       integer NOT NULL DEFAULT 0,
    error          text,
    -- Per-Source progress, shown in the activity panel.
    progress       jsonb NOT NULL DEFAULT '{}',
    -- Intermediate generation result, so a retry resumes instead of re-paying
    -- for the LLM calls. Becomes the Changeset in phase 3.
    generation     jsonb,
    created_by     uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    started_at     timestamptz,
    finished_at    timestamptz
);
CREATE INDEX ingest_jobs_project_status_idx ON ingest_jobs (project_id, status, created_at);
-- One live job per Source; a finished one does not block re-ingest.
CREATE UNIQUE INDEX ingest_jobs_active_source_key
    ON ingest_jobs (project_id, source_path) WHERE status IN ('pending', 'processing');

ALTER TABLE page_versions
    ADD CONSTRAINT page_versions_ingest_job_fk
    FOREIGN KEY (ingest_job_id) REFERENCES ingest_jobs (id) ON DELETE SET NULL;

-- Replaces ingest-warnings.log.
CREATE TABLE ingest_events (
    id         bigserial PRIMARY KEY,
    job_id     uuid NOT NULL REFERENCES ingest_jobs (id) ON DELETE CASCADE,
    level      text NOT NULL CHECK (level IN ('info', 'warning', 'error')),
    message    text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX ingest_events_job_idx ON ingest_events (job_id, id);

-- Replaces ingest-cache.json: skip a Source whose content hash is unchanged.
CREATE TABLE ingest_cache (
    project_id    uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    source_path   text NOT NULL,
    content_hash  text NOT NULL,
    files_written text[] NOT NULL DEFAULT '{}',
    ingested_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, source_path)
);

-- Replaces image-caption-cache.json.
CREATE TABLE image_captions (
    project_id      uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    image_hash      text NOT NULL,
    output_language text NOT NULL DEFAULT '',
    caption         text NOT NULL,
    mime_type       text,
    model           text NOT NULL,
    captured_at     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, image_hash, output_language)
);

-- Replaces dedup-queue.json.
CREATE TABLE dedup_tasks (
    id             text NOT NULL,
    project_id     uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    duplicate_group jsonb NOT NULL,
    canonical_slug text NOT NULL,
    status         text NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'processing', 'done', 'failed')),
    retry_count    integer NOT NULL DEFAULT 0,
    error          text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
);
CREATE INDEX dedup_tasks_status_idx ON dedup_tasks (project_id, status);

-- Replaces dedup-not-duplicates.json. (page_a, page_b) is stored ordered so a
-- pair has exactly one row.
CREATE TABLE dedup_exclusions (
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    page_a     text NOT NULL,
    page_b     text NOT NULL,
    marked_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, page_a, page_b),
    CHECK (page_a < page_b)
);

-- Replaces review.json and review-inbox.json. Shared by every Editor of the
-- Project; `resolved_by` records who acted. Not a Changeset: never blocks a commit.
CREATE TABLE review_items (
    project_id      uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    -- Content-derived and stable across regeneration (review-store.ts), which
    -- is also what makes appending the same item twice idempotent.
    id              text NOT NULL,
    type            text NOT NULL
                    CHECK (type IN ('contradiction', 'duplicate', 'missing-page', 'confirm', 'suggestion')),
    title           text NOT NULL,
    description     text NOT NULL DEFAULT '',
    source_path     text,
    affected_pages  text[] NOT NULL DEFAULT '{}',
    search_queries  text[] NOT NULL DEFAULT '{}',
    options         jsonb NOT NULL DEFAULT '[]',
    resolved        boolean NOT NULL DEFAULT false,
    resolved_action text,
    resolved_by     uuid REFERENCES users (id) ON DELETE SET NULL,
    resolved_at     timestamptz,
    ingest_job_id   uuid REFERENCES ingest_jobs (id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
);
CREATE INDEX review_items_open_idx ON review_items (project_id, created_at) WHERE NOT resolved;

-- Replaces lint.json. A run replaces the Project's previous findings.
CREATE TABLE lint_findings (
    id               bigserial PRIMARY KEY,
    project_id       uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    type             text NOT NULL CHECK (type IN ('orphan', 'broken-link', 'no-outlinks', 'semantic')),
    severity         text NOT NULL CHECK (severity IN ('warning', 'info')),
    page_path        text NOT NULL,
    detail           text NOT NULL,
    affected_pages   text[] NOT NULL DEFAULT '{}',
    broken_target    text,
    suggested_target text,
    suggested_source text,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX lint_findings_project_idx ON lint_findings (project_id);

-- Replaces conversations.json and chats/<id>.json. Private to the user who
-- started them; Project membership decides whether they can start one.
CREATE TABLE conversations (
    project_id      uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    id              text NOT NULL,
    user_id         uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    title           text NOT NULL,
    selected_skills text[] NOT NULL DEFAULT '{}',
    context_files   text[] NOT NULL DEFAULT '{}',
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
);
CREATE INDEX conversations_user_idx ON conversations (project_id, user_id, updated_at DESC);

CREATE TABLE chat_messages (
    project_id         uuid NOT NULL,
    conversation_id    text NOT NULL,
    id                 text NOT NULL,
    seq                bigserial NOT NULL,
    role               text NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content            text NOT NULL,
    -- Assistant-turn metadata, kept as the client sends it.
    "references"       jsonb,
    agent_steps        jsonb,
    agent_file_changes jsonb,
    user_input_request jsonb,
    -- Vision attachments. Inline base64 for now; move to `blobs` if they grow.
    images             jsonb,
    context_files      text[] NOT NULL DEFAULT '{}',
    created_at         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, conversation_id, id),
    FOREIGN KEY (project_id, conversation_id)
        REFERENCES conversations (project_id, id) ON DELETE CASCADE
);
CREATE INDEX chat_messages_order_idx ON chat_messages (project_id, conversation_id, seq);

-- Replaces `.llm-wiki/agent-sessions/<id>.json`: the agent's short-term memory
-- across turns. The message list is small and always read and written whole,
-- so it stays one jsonb document.
CREATE TABLE agent_sessions (
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    id         text NOT NULL,
    user_id    uuid REFERENCES users (id) ON DELETE CASCADE,
    messages   jsonb NOT NULL DEFAULT '[]',
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
);
CREATE INDEX agent_sessions_user_idx ON agent_sessions (project_id, user_id, updated_at DESC);
