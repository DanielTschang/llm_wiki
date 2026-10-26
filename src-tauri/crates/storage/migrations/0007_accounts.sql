-- Password accounts and browser sessions.
--
-- `users` (0001) is the identity; this adds the one way to sign in that exists
-- today, a username and password. An IdP (OIDC, LDAP) would sign users in
-- through `users.external_subject` instead and leave this table empty for them.

CREATE TABLE user_credentials (
    user_id             uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    username            text NOT NULL CHECK (username ~ '^[a-z0-9][a-z0-9._-]{2,63}$'),
    -- Argon2id, PHC string format (parameters and salt are inside it).
    password_hash       text NOT NULL,
    -- Consecutive wrong passwords; after enough, sign-in is refused until
    -- `locked_until`. A correct password resets both.
    failed_attempts     integer NOT NULL DEFAULT 0,
    locked_until        timestamptz,
    password_changed_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX user_credentials_username_key ON user_credentials (username);

-- Only the SHA-256 of the cookie value is stored, so a database leak does not
-- hand out live sessions.
CREATE TABLE sessions (
    token_hash   bytea PRIMARY KEY,
    user_id      uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);
CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_expiry_idx ON sessions (expires_at);
