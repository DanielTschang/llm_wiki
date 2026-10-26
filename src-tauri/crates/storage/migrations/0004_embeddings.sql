-- pgvector, one chunk table per embedding model (replaces lancedb/ and
-- embedding-revisions/). A pgvector column has a fixed dimension, so the
-- model's dimension is part of its table. Chunk rows mirror the LanceDB
-- `wiki_chunks_v2` table, so the swap is a drop-in.

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE embedding_models (
    id         serial PRIMARY KEY,
    -- Provider-qualified model name as configured, e.g. 'openai/text-embedding-3-small'.
    model      text NOT NULL UNIQUE,
    dimensions integer NOT NULL CHECK (dimensions > 0),
    table_name text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Replaces embedding-revisions/<page>.json: which content (and embedding
-- config) a page was last embedded from, so unchanged pages are skipped.
CREATE TABLE page_embedding_state (
    project_id  uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    page_id     uuid NOT NULL REFERENCES pages (id) ON DELETE CASCADE,
    model_id    integer NOT NULL REFERENCES embedding_models (id) ON DELETE CASCADE,
    fingerprint text NOT NULL,
    embedded_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (page_id, model_id)
);
CREATE INDEX page_embedding_state_project_idx ON page_embedding_state (project_id, model_id);

-- Registers a model and creates its chunk table and ANN index. Idempotent.
-- Up to 2000 dims uses `vector`; up to 4000 uses `halfvec` (the HNSW limit of
-- each type). Distance is L2, which is what LanceDB used, so the existing
-- `score = 1 / (1 + distance)` convention and its thresholds carry over.
CREATE FUNCTION register_embedding_model(p_model text, p_dimensions integer)
RETURNS integer
LANGUAGE plpgsql AS $$
DECLARE
    v_id    integer;
    v_dims  integer;
    v_table text;
    v_type  text;
    v_ops   text;
BEGIN
    SELECT id, dimensions INTO v_id, v_dims FROM embedding_models WHERE model = p_model;
    IF FOUND THEN
        IF v_dims <> p_dimensions THEN
            RAISE EXCEPTION 'embedding model % is registered with % dimensions, not %',
                p_model, v_dims, p_dimensions;
        END IF;
        RETURN v_id;
    END IF;

    IF p_dimensions <= 2000 THEN
        v_type := format('vector(%s)', p_dimensions);
        v_ops := 'vector_l2_ops';
    ELSIF p_dimensions <= 4000 THEN
        v_type := format('halfvec(%s)', p_dimensions);
        v_ops := 'halfvec_l2_ops';
    ELSE
        RAISE EXCEPTION 'embedding dimension % exceeds the 4000 pgvector HNSW limit', p_dimensions;
    END IF;

    INSERT INTO embedding_models (model, dimensions, table_name)
    VALUES (p_model, p_dimensions, 'pending')
    RETURNING id INTO v_id;
    v_table := format('chunks_m%s', v_id);
    UPDATE embedding_models SET table_name = v_table WHERE id = v_id;

    EXECUTE format($t$
        CREATE TABLE %I (
            project_id   uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
            page_id      uuid NOT NULL REFERENCES pages (id) ON DELETE CASCADE,
            chunk_index  integer NOT NULL,
            chunk_text   text NOT NULL,
            heading_path text NOT NULL DEFAULT '',
            embedding    %s NOT NULL,
            PRIMARY KEY (page_id, chunk_index)
        )$t$, v_table, v_type);
    EXECUTE format('CREATE INDEX %I ON %I USING hnsw (embedding %s)',
                   v_table || '_hnsw', v_table, v_ops);
    EXECUTE format('CREATE INDEX %I ON %I (project_id)', v_table || '_project', v_table);
    RETURN v_id;
END $$;
