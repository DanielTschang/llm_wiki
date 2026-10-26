//! Password accounts and sessions.
//!
//! Anyone who can reach the server is a stranger until they sign in, and a
//! user is who a session cookie says they are. Passwords are Argon2id hashes,
//! repeated failures lock an account for a while, and only the SHA-256 of a
//! session token is stored.

use std::sync::OnceLock;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::sql::{backend, is_unique_violation};
use crate::StoreError;

/// Wrong passwords in a row before sign-in is refused for a while.
const MAX_FAILED_ATTEMPTS: i32 = 5;
const LOCKOUT_MINUTES: i32 = 15;
/// A session lasts this long after its last use.
const SESSION_DAYS: i32 = 30;
const MIN_PASSWORD_CHARS: usize = 8;
/// Hashing cost grows with length; this keeps a huge "password" from being
/// a way to burn CPU.
const MAX_PASSWORD_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    /// `None` for accounts that do not sign in with a password.
    pub username: Option<String>,
    pub display_name: String,
    pub email: Option<String>,
    pub is_platform_admin: bool,
    pub disabled: bool,
    /// A Service Account issued to a Plugin rather than a person.
    pub is_service: bool,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub password: String,
    pub is_platform_admin: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthError {
    /// Deliberately the same for an unknown user and a wrong password.
    #[error("wrong username or password")]
    InvalidCredentials,
    #[error("too many failed attempts; try again after {until}")]
    Locked { until: DateTime<Utc> },
    #[error("this account is disabled")]
    Disabled,
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Lower-cased username, or why it cannot be one.
pub fn normalize_username(raw: &str) -> Result<String, StoreError> {
    let name = raw.trim().to_lowercase();
    let valid = (3..=64).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if valid {
        Ok(name)
    } else {
        Err(StoreError::Invalid(
            "a username is 3 to 64 letters, digits, '.', '_' or '-', starting with a letter or digit"
                .into(),
        ))
    }
}

pub fn check_password(password: &str) -> Result<(), StoreError> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(StoreError::Invalid(format!(
            "a password needs at least {MIN_PASSWORD_CHARS} characters"
        )));
    }
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(StoreError::Invalid("that password is too long".into()));
    }
    Ok(())
}

fn hash_password(password: &str) -> Result<String, StoreError> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
        .map_err(|error| StoreError::Backend(format!("hash password: {error}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// A hash to check against when the username does not exist, so an unknown
/// user costs the same time as a wrong password.
fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password("not a real password").expect("hashing works"))
}

/// Runs the slow hash off the async worker threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, StoreError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| StoreError::Backend(format!("hashing task failed: {error}")))
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

const USER_COLUMNS: &str = "u.id, u.display_name, u.email, u.is_platform_admin,
     u.disabled_at IS NOT NULL AS disabled, u.kind = 'service' AS is_service, c.username";

fn user_from_row(row: &PgRow) -> Result<User, StoreError> {
    Ok(User {
        id: row.try_get("id").map_err(backend)?,
        username: row.try_get("username").map_err(backend)?,
        display_name: row.try_get("display_name").map_err(backend)?,
        email: row.try_get("email").map_err(backend)?,
        is_platform_admin: row.try_get("is_platform_admin").map_err(backend)?,
        disabled: row.try_get("disabled").map_err(backend)?,
        is_service: row.try_get("is_service").map_err(backend)?,
    })
}

#[derive(Clone)]
pub struct Accounts {
    pool: PgPool,
}

impl Accounts {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(&self, new: NewUser) -> Result<User, StoreError> {
        let username = normalize_username(&new.username)?;
        check_password(&new.password)?;
        let display_name = new.display_name.trim();
        if display_name.is_empty() {
            return Err(StoreError::Invalid("a display name is required".into()));
        }
        let password = new.password.clone();
        let hash = blocking(move || hash_password(&password)).await??;

        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (display_name, email, is_platform_admin)
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(display_name)
        .bind(
            new.email
                .as_deref()
                .map(str::trim)
                .filter(|e| !e.is_empty()),
        )
        .bind(new.is_platform_admin)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                StoreError::AlreadyExists("that email is already in use".into())
            } else {
                backend(error)
            }
        })?;
        sqlx::query(
            "INSERT INTO user_credentials (user_id, username, password_hash) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(&username)
        .bind(&hash)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                StoreError::AlreadyExists(format!("the username {username} is taken"))
            } else {
                backend(error)
            }
        })?;
        tx.commit().await.map_err(backend)?;
        self.user(id)
            .await?
            .ok_or_else(|| StoreError::Backend("the new user vanished".into()))
    }

    pub async fn user(&self, id: Uuid) -> Result<Option<User>, StoreError> {
        let row = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM users u
             LEFT JOIN user_credentials c ON c.user_id = u.id WHERE u.id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.as_ref().map(user_from_row).transpose()
    }

    pub async fn list_users(&self) -> Result<Vec<User>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM users u
             LEFT JOIN user_credentials c ON c.user_id = u.id
             ORDER BY lower(u.display_name), u.created_at"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(user_from_row).collect()
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, StoreError> {
        let Ok(username) = normalize_username(username) else {
            return Ok(None);
        };
        let row = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM users u
             JOIN user_credentials c ON c.user_id = u.id WHERE c.username = $1"
        ))
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.as_ref().map(user_from_row).transpose()
    }

    /// Checks a username and password. Counts failures and locks the account
    /// after too many; an unknown username takes as long as a wrong password.
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<User, AuthError> {
        let username = username.trim().to_lowercase();
        let row = sqlx::query(
            "SELECT c.password_hash, c.locked_until, c.user_id, u.disabled_at IS NOT NULL AS disabled
             FROM user_credentials c JOIN users u ON u.id = c.user_id WHERE c.username = $1",
        )
        .bind(&username)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else {
            let password = password.to_string();
            blocking(move || verify_password(&password, dummy_hash())).await?;
            return Err(AuthError::InvalidCredentials);
        };
        let user_id: Uuid = row.try_get("user_id").map_err(backend)?;
        let locked_until: Option<DateTime<Utc>> = row.try_get("locked_until").map_err(backend)?;
        if let Some(until) = locked_until.filter(|until| *until > Utc::now()) {
            return Err(AuthError::Locked { until });
        }
        let hash: String = row.try_get("password_hash").map_err(backend)?;
        let candidate = password.to_string();
        let correct = blocking(move || verify_password(&candidate, &hash)).await?;
        if !correct {
            sqlx::query(
                "UPDATE user_credentials SET
                     failed_attempts = failed_attempts + 1,
                     locked_until = CASE WHEN failed_attempts + 1 >= $2
                         THEN now() + make_interval(mins => $3) ELSE locked_until END
                 WHERE user_id = $1",
            )
            .bind(user_id)
            .bind(MAX_FAILED_ATTEMPTS)
            .bind(LOCKOUT_MINUTES)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
            return Err(AuthError::InvalidCredentials);
        }
        if row.try_get::<bool, _>("disabled").map_err(backend)? {
            return Err(AuthError::Disabled);
        }
        sqlx::query(
            "UPDATE user_credentials SET failed_attempts = 0, locked_until = NULL WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(self
            .user(user_id)
            .await?
            .ok_or_else(|| StoreError::Backend("the user vanished".into()))?)
    }

    /// Changes a password and signs the user out everywhere.
    pub async fn set_password(&self, user_id: Uuid, password: &str) -> Result<(), StoreError> {
        check_password(password)?;
        let password = password.to_string();
        let hash = blocking(move || hash_password(&password)).await??;
        let changed = sqlx::query(
            "UPDATE user_credentials SET password_hash = $2, failed_attempts = 0,
                 locked_until = NULL, password_changed_at = now() WHERE user_id = $1",
        )
        .bind(user_id)
        .bind(hash)
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound(format!("user {user_id}")));
        }
        self.revoke_user_sessions(user_id).await
    }

    /// Disabling signs the user out everywhere. The last active Platform
    /// Admin cannot be disabled: nobody could undo it.
    pub async fn set_disabled(&self, user_id: Uuid, disabled: bool) -> Result<(), StoreError> {
        if disabled {
            self.require_another_admin(user_id).await?;
        }
        let changed = sqlx::query(
            "UPDATE users SET disabled_at = CASE WHEN $2 THEN coalesce(disabled_at, now()) END
             WHERE id = $1",
        )
        .bind(user_id)
        .bind(disabled)
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound(format!("user {user_id}")));
        }
        if disabled {
            self.revoke_user_sessions(user_id).await?;
        }
        Ok(())
    }

    pub async fn set_platform_admin(&self, user_id: Uuid, admin: bool) -> Result<(), StoreError> {
        if !admin {
            self.require_another_admin(user_id).await?;
        }
        let changed = sqlx::query("UPDATE users SET is_platform_admin = $2 WHERE id = $1")
            .bind(user_id)
            .bind(admin)
            .execute(&self.pool)
            .await
            .map_err(backend)?
            .rows_affected();
        if changed == 0 {
            return Err(StoreError::NotFound(format!("user {user_id}")));
        }
        Ok(())
    }

    /// Fails unless an active Platform Admin other than `user_id` exists.
    async fn require_another_admin(&self, user_id: Uuid) -> Result<(), StoreError> {
        let target_is_admin: bool = sqlx::query_scalar(
            "SELECT is_platform_admin AND disabled_at IS NULL FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?
        .unwrap_or(false);
        if !target_is_admin {
            return Ok(());
        }
        let others: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM users
             WHERE is_platform_admin AND disabled_at IS NULL AND kind = 'human' AND id <> $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        if others == 0 {
            return Err(StoreError::Forbidden(
                "there must always be one active Platform Admin".into(),
            ));
        }
        Ok(())
    }

    /// Creates the first Platform Admin if the platform has none yet.
    /// `Ok(None)` means one already exists and nothing was created.
    pub async fn ensure_bootstrap_admin(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<User>, StoreError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM users
             WHERE is_platform_admin AND disabled_at IS NULL AND kind = 'human')",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        if exists {
            return Ok(None);
        }
        self.create_user(NewUser {
            username: username.to_string(),
            display_name: "Administrator".to_string(),
            email: None,
            password: password.to_string(),
            is_platform_admin: true,
        })
        .await
        .map(Some)
    }

    /// A new browser session for `user_id`; the returned string is the cookie
    /// value and is not recoverable later.
    pub async fn create_session(&self, user_id: Uuid) -> Result<String, StoreError> {
        let token = new_token();
        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, expires_at)
             VALUES ($1, $2, now() + make_interval(days => $3))",
        )
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(SESSION_DAYS)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(token)
    }

    /// The user a session cookie belongs to, if it is live and the account is
    /// active. Use extends the session.
    pub async fn user_for_session(&self, token: &str) -> Result<Option<User>, StoreError> {
        let hash = hash_token(token);
        let row = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM sessions s
             JOIN users u ON u.id = s.user_id
             LEFT JOIN user_credentials c ON c.user_id = u.id
             WHERE s.token_hash = $1 AND s.expires_at > now() AND u.disabled_at IS NULL"
        ))
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        // Sliding expiry, written at most hourly so reads stay reads.
        sqlx::query(
            "UPDATE sessions SET last_seen_at = now(),
                 expires_at = now() + make_interval(days => $2)
             WHERE token_hash = $1 AND last_seen_at < now() - interval '1 hour'",
        )
        .bind(&hash)
        .bind(SESSION_DAYS)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        user_from_row(&row).map(Some)
    }

    pub async fn revoke_session(&self, token: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_token(token))
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    pub async fn revoke_user_sessions(&self, user_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    /// Drops expired sessions; run now and then.
    pub async fn purge_expired_sessions(&self) -> Result<u64, StoreError> {
        Ok(
            sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
                .execute(&self.pool)
                .await
                .map_err(backend)?
                .rows_affected(),
        )
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;

    #[test]
    fn usernames_are_lowercased_and_restricted() {
        assert_eq!(normalize_username("  Alice.Wu ").unwrap(), "alice.wu");
        for bad in [
            "",
            "ab",
            "-lead",
            ".lead",
            "has space",
            "slash/name",
            "ünï",
            &"a".repeat(65),
        ] {
            assert!(normalize_username(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn passwords_have_a_floor_and_a_ceiling() {
        assert!(check_password("short").is_err());
        assert!(check_password("长长长长长长长长").is_ok());
        assert!(check_password("eight chars ok").is_ok());
        assert!(check_password(&"x".repeat(MAX_PASSWORD_BYTES + 1)).is_err());
    }

    #[test]
    fn hashes_verify_and_are_salted() {
        let a = hash_password("correct horse").unwrap();
        let b = hash_password("correct horse").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("$argon2id$"));
        assert!(verify_password("correct horse", &a));
        assert!(!verify_password("wrong horse", &a));
        assert!(!verify_password("x", "not a hash"));
    }

    #[test]
    fn session_tokens_are_long_unique_and_hashed_before_storage() {
        let (a, b) = (new_token(), new_token());
        assert_ne!(a, b);
        assert!(a.len() >= 43);
        assert_eq!(hash_token(&a).len(), 32);
        assert_ne!(hash_token(&a), a.as_bytes());
    }

    async fn accounts() -> Option<Accounts> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        Some(Accounts::new(pool))
    }

    fn unique(prefix: &str) -> String {
        format!("{prefix}-{}", &Uuid::new_v4().simple().to_string()[..10])
    }

    async fn user(accounts: &Accounts, admin: bool) -> (User, String, String) {
        let name = unique("user");
        let password = "a fine password".to_string();
        let user = accounts
            .create_user(NewUser {
                username: name.clone(),
                display_name: format!("Test {name}"),
                email: None,
                password: password.clone(),
                is_platform_admin: admin,
            })
            .await
            .unwrap();
        (user, name, password)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_user_signs_in_with_their_password_in_any_letter_case() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (created, name, password) = user(&accounts, false).await;
        assert_eq!(created.username.as_deref(), Some(name.as_str()));
        assert!(!created.is_platform_admin && !created.disabled && !created.is_service);

        let signed_in = accounts
            .authenticate(&name.to_uppercase(), &password)
            .await
            .unwrap();
        assert_eq!(signed_in.id, created.id);
        assert_eq!(
            accounts.authenticate(&name, "not the password").await,
            Err(AuthError::InvalidCredentials)
        );
        assert_eq!(
            accounts
                .authenticate("nobody-by-this-name", &password)
                .await,
            Err(AuthError::InvalidCredentials)
        );
        assert_eq!(
            accounts
                .find_by_username(&name)
                .await
                .unwrap()
                .map(|u| u.id),
            Some(created.id)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn usernames_and_emails_are_unique_and_input_is_checked() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (_, name, _) = user(&accounts, false).await;
        let again = accounts
            .create_user(NewUser {
                username: name.to_uppercase(),
                display_name: "Other".into(),
                email: None,
                password: "another password".into(),
                is_platform_admin: false,
            })
            .await;
        assert!(
            matches!(again, Err(StoreError::AlreadyExists(_))),
            "{again:?}"
        );
        for (username, password, display) in [
            ("ok-name-1", "short", "Name"),
            ("x", "a fine password", "Name"),
            ("ok-name-2", "a fine password", "   "),
        ] {
            let result = accounts
                .create_user(NewUser {
                    username: username.into(),
                    display_name: display.into(),
                    email: None,
                    password: password.into(),
                    is_platform_admin: false,
                })
                .await;
            assert!(
                matches!(result, Err(StoreError::Invalid(_))),
                "{username}: {result:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn repeated_wrong_passwords_lock_the_account_until_it_expires() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (created, name, password) = user(&accounts, false).await;
        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert_eq!(
                accounts.authenticate(&name, "wrong password").await,
                Err(AuthError::InvalidCredentials)
            );
        }
        // Locked now, even for the right password.
        assert!(matches!(
            accounts.authenticate(&name, &password).await,
            Err(AuthError::Locked { .. })
        ));
        // The lock lapses by itself; a correct password then clears the count.
        sqlx::query("UPDATE user_credentials SET locked_until = now() - interval '1 second' WHERE user_id = $1")
            .bind(created.id)
            .execute(&accounts.pool)
            .await
            .unwrap();
        assert_eq!(
            accounts.authenticate(&name, &password).await.unwrap().id,
            created.id
        );
        let attempts: i32 =
            sqlx::query_scalar("SELECT failed_attempts FROM user_credentials WHERE user_id = $1")
                .bind(created.id)
                .fetch_one(&accounts.pool)
                .await
                .unwrap();
        assert_eq!(attempts, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_session_identifies_its_user_until_revoked_or_expired() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (created, _, _) = user(&accounts, false).await;
        let token = accounts.create_session(created.id).await.unwrap();
        assert_eq!(
            accounts
                .user_for_session(&token)
                .await
                .unwrap()
                .map(|u| u.id),
            Some(created.id)
        );
        assert!(accounts
            .user_for_session("not-a-token")
            .await
            .unwrap()
            .is_none());

        accounts.revoke_session(&token).await.unwrap();
        assert!(accounts.user_for_session(&token).await.unwrap().is_none());

        let second = accounts.create_session(created.id).await.unwrap();
        sqlx::query(
            "UPDATE sessions SET expires_at = now() - interval '1 minute' WHERE user_id = $1",
        )
        .bind(created.id)
        .execute(&accounts.pool)
        .await
        .unwrap();
        assert!(accounts.user_for_session(&second).await.unwrap().is_none());
        assert!(accounts.purge_expired_sessions().await.unwrap() >= 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn changing_a_password_or_disabling_an_account_ends_its_sessions() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (created, name, password) = user(&accounts, false).await;
        let token = accounts.create_session(created.id).await.unwrap();

        accounts
            .set_password(created.id, "a brand new password")
            .await
            .unwrap();
        assert!(accounts.user_for_session(&token).await.unwrap().is_none());
        assert_eq!(
            accounts.authenticate(&name, &password).await,
            Err(AuthError::InvalidCredentials)
        );
        accounts
            .authenticate(&name, "a brand new password")
            .await
            .unwrap();

        let token = accounts.create_session(created.id).await.unwrap();
        accounts.set_disabled(created.id, true).await.unwrap();
        assert!(accounts.user_for_session(&token).await.unwrap().is_none());
        assert_eq!(
            accounts.authenticate(&name, "a brand new password").await,
            Err(AuthError::Disabled)
        );
        accounts.set_disabled(created.id, false).await.unwrap();
        accounts
            .authenticate(&name, "a brand new password")
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_bootstrap_admin_is_only_created_when_there_is_no_admin() {
        let Some(accounts) = accounts().await else {
            return;
        };
        // Whatever else is in the database, an admin exists after this.
        user(&accounts, true).await;
        let made = accounts
            .ensure_bootstrap_admin(&unique("boot"), "a fine password")
            .await
            .unwrap();
        assert!(made.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn an_admin_can_be_demoted_while_another_remains() {
        let Some(accounts) = accounts().await else {
            return;
        };
        let (a, _, _) = user(&accounts, true).await;
        let (_b, _, _) = user(&accounts, true).await;
        accounts.set_platform_admin(a.id, false).await.unwrap();
        assert!(
            !accounts
                .user(a.id)
                .await
                .unwrap()
                .unwrap()
                .is_platform_admin
        );
        assert!(matches!(
            accounts.set_disabled(Uuid::new_v4(), true).await,
            Err(StoreError::NotFound(_))
        ));
    }
}
