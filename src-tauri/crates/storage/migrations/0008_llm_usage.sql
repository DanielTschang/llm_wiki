-- One row per call through the LLM gateway. Statistics only: nothing here
-- limits anyone, and no prompt or reply text is kept.
CREATE TABLE llm_usage (
    id            bigserial PRIMARY KEY,
    created_at    timestamptz NOT NULL DEFAULT now(),
    -- The account that asked. A Service Account or the operator's token shows
    -- as that account; the ingest worker, which acts for no one, as NULL.
    user_id       uuid REFERENCES users (id) ON DELETE SET NULL,
    -- The Project the call was for, when the caller said which. The row stays
    -- after the Project is deleted: the cost happened.
    project_id    uuid,
    model_id      text NOT NULL,
    status        integer NOT NULL,
    duration_ms   integer NOT NULL,
    bytes_in      bigint NOT NULL DEFAULT 0,
    bytes_out     bigint NOT NULL DEFAULT 0,
    -- As reported by the provider when the reply carries it.
    input_tokens  bigint,
    output_tokens bigint
);
CREATE INDEX llm_usage_time_idx ON llm_usage (created_at DESC);
CREATE INDEX llm_usage_user_idx ON llm_usage (user_id, created_at DESC);
CREATE INDEX llm_usage_project_idx ON llm_usage (project_id, created_at DESC);
