-- The `.llm-wiki/*` state files of a Project, kept as documents.
--
-- The ingest worker and the browser still read and write these as whole JSON
-- files through the file commands (review items, conversations, the ingest
-- queue, caches, lint results, ...). This table is what those paths resolve to
-- on a platform server, so they keep working unchanged while the typed tables
-- in 0003 take over file by file. A file moves out of here when its table has
-- a repository and the code that wrote the JSON has been rewritten.
--
-- Not history: page backups are `page_versions`, and a deleted file is simply
-- gone. `version` is the optimistic-lock counter the file interface exposes.
CREATE TABLE state_files (
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    path       text NOT NULL CHECK (path !~ '(^/|\.\./|//)'),
    content    text NOT NULL,
    version    integer NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, path)
);

-- Directory listings and subtree deletes are prefix scans.
CREATE INDEX state_files_prefix_idx ON state_files (project_id, path text_pattern_ops);
