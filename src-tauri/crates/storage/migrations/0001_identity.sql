-- Identity, Projects and Project-level permissions.
-- Permissions exist only at Project level (CONTEXT.md); there is no per-page ACL.

CREATE TABLE users (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 'service' rows are Service Accounts issued to Plugins; they take Project
    -- roles exactly like humans do.
    kind              text NOT NULL DEFAULT 'human' CHECK (kind IN ('human', 'service')),
    -- Identity as asserted by the organisation's IdP (OIDC `sub`, LDAP DN, ...).
    -- NULL for service accounts and for the phase-1 bootstrap user.
    external_subject  text UNIQUE,
    email             text,
    display_name      text NOT NULL,
    is_platform_admin boolean NOT NULL DEFAULT false,
    disabled_at       timestamptz,
    created_at        timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_key ON users (lower(email)) WHERE email IS NOT NULL;

CREATE TABLE projects (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name       text NOT NULL CHECK (btrim(name) <> ''),
    created_by uuid REFERENCES users (id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE project_members (
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    user_id    uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       text NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
    added_by   uuid REFERENCES users (id),
    added_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, user_id)
);
CREATE INDEX project_members_user_idx ON project_members (user_id);

-- Replaces the `app-state.json` Project-scoped keys, `lint-config.json` and
-- `history-settings.json`, and the per-project LLM override in `project.json`.
-- Secrets (API keys) never live in the database; see plans/schema.md.
CREATE TABLE project_settings (
    project_id uuid NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    key        text NOT NULL,
    value      jsonb NOT NULL,
    updated_by uuid REFERENCES users (id),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, key)
);

-- Replaces `chat-preferences.json` and the user-scoped keys of `app-state.json`.
CREATE TABLE user_preferences (
    user_id    uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    key        text NOT NULL,
    value      jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, key)
);

-- Organisation-wide, non-secret configuration (model catalogue, base URLs,
-- quota thresholds). Edited by Platform Admins only.
CREATE TABLE server_settings (
    key        text PRIMARY KEY,
    value      jsonb NOT NULL,
    updated_by uuid REFERENCES users (id),
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- Append-only record of who did what (membership changes, uploads, deletes,
-- settings edits, ...). Page content history lives in `page_versions`.
-- `project_id` is NULL for platform-level events.
CREATE TABLE activity_log (
    id          bigserial PRIMARY KEY,
    project_id  uuid REFERENCES projects (id) ON DELETE CASCADE,
    actor_id    uuid REFERENCES users (id) ON DELETE SET NULL,
    action      text NOT NULL,
    target_type text,
    target_id   text,
    detail      jsonb NOT NULL DEFAULT '{}',
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX activity_log_project_idx ON activity_log (project_id, id DESC);
CREATE INDEX activity_log_actor_idx ON activity_log (actor_id, id DESC);
