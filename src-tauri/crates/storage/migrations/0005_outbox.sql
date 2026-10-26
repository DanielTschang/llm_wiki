-- Transactional outbox (ADR-0003). A state change and the event announcing it
-- are inserted in one transaction; a relay publishes unpublished rows to
-- JetStream afterwards. NATS being down delays events, never loses them.
CREATE TABLE outbox (
    id           bigserial PRIMARY KEY,
    -- JetStream subject, e.g. 'ingest.job.requested', 'wiki.page.committed'.
    subject      text NOT NULL,
    -- Sent as Nats-Msg-Id so a relay retry after a lost ack is deduplicated
    -- by the stream instead of delivered twice.
    msg_id       text NOT NULL UNIQUE DEFAULT gen_random_uuid()::text,
    project_id   uuid REFERENCES projects (id) ON DELETE CASCADE,
    payload      jsonb NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    published_at timestamptz,
    attempts     integer NOT NULL DEFAULT 0,
    last_error   text
);
-- The relay's work queue: oldest unpublished first, taken with FOR UPDATE SKIP LOCKED.
CREATE INDEX outbox_unpublished_idx ON outbox (id) WHERE published_at IS NULL;
-- Published rows are deleted by a periodic job once past the stream's dedup window.
CREATE INDEX outbox_published_idx ON outbox (published_at) WHERE published_at IS NOT NULL;
