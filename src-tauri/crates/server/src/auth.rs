//! Credentials of a request.
//!
//! Browsers sign in with an account and get an `HttpOnly`, `SameSite=Strict`
//! session cookie (see `Accounts::create_session`); scripts and the ingest
//! worker send `Authorization: Bearer`. Cookie-authenticated state-changing
//! requests must also carry [`CLIENT_HEADER`]: a cross-site page cannot add a custom header without a
//! CORS preflight, which this server never approves.

use std::fs;
use std::io::Write;
use std::path::Path;

use axum::http::{header, HeaderMap};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const COOKIE_NAME: &str = "llm_wiki_session";
pub const CLIENT_HEADER: &str = "x-llm-wiki-client";
const TOKEN_FILE: &str = "server-token";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    Cookie,
    Bearer,
    /// The ingest worker this server spawned (its own per-run token).
    Worker,
    None,
}

pub struct Auth {
    token_digest: [u8; 32],
    worker_token_digest: Option<[u8; 32]>,
    secure_cookie: bool,
}

impl Auth {
    pub fn new(token: &str, secure_cookie: bool) -> Self {
        let token_digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        Self {
            token_digest,
            worker_token_digest: None,
            secure_cookie,
        }
    }

    /// Accept `token` as the worker credential. Kept separate from the
    /// owner token so the owner's secret is never handed to a subprocess.
    pub fn with_worker_token(mut self, token: &str) -> Self {
        self.worker_token_digest = Some(Sha256::digest(token.as_bytes()).into());
        self
    }


    /// The Bearer credential, if the request carries one that matches the
    /// operator token or the worker's.
    pub fn bearer_credential(&self, headers: &HeaderMap) -> Credential {
        if let Some(bearer) = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
        {
            let digest: [u8; 32] = Sha256::digest(bearer.trim().as_bytes()).into();
            if bool::from(digest.ct_eq(&self.token_digest)) {
                return Credential::Bearer;
            }
            if self
                .worker_token_digest
                .is_some_and(|worker| bool::from(digest.ct_eq(&worker)))
            {
                return Credential::Worker;
            }
        }
        Credential::None
    }

    /// A session cookie carrying `value`, the per-user token of a platform
    /// server.
    pub fn cookie_for(&self, value: &str) -> String {
        format!(
            "{COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000{}",
            if self.secure_cookie { "; Secure" } else { "" }
        )
    }


    pub fn clear_cookie(&self) -> String {
        format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
    }
}

/// The value of the session cookie, if the request has one.
pub fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value.to_string())
}

/// Returns the configured token, or the persisted/generated one and
/// whether it was generated just now.
pub fn load_or_create_token(
    data_dir: &Path,
    configured: Option<String>,
) -> Result<(String, bool), String> {
    if let Some(token) = configured.filter(|t| !t.trim().is_empty()) {
        return Ok((token.trim().to_string(), false));
    }
    let path = data_dir.join(TOKEN_FILE);
    if let Ok(existing) = fs::read_to_string(&path) {
        if !existing.trim().is_empty() {
            return Ok((existing.trim().to_string(), false));
        }
    }
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    file.write_all(token.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok((token, true))
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(header::HeaderName, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(name.clone(), HeaderValue::from_str(value).unwrap());
        }
        map
    }

    #[test]
    fn accepts_the_operator_token_and_the_workers() {
        let auth = Auth::new("owner", false).with_worker_token("worker");
        let bearer = |value: &str| {
            auth.bearer_credential(&headers(&[(header::AUTHORIZATION, value)]))
        };
        assert_eq!(bearer("Bearer owner"), Credential::Bearer);
        assert_eq!(bearer("Bearer worker"), Credential::Worker);
        assert_eq!(bearer("Bearer wrong"), Credential::None);
        assert_eq!(
            auth.bearer_credential(&headers(&[(header::COOKIE, "llm_wiki_session=owner")])),
            Credential::None,
            "a cookie is not a bearer credential"
        );
    }

    #[test]
    fn generated_token_is_persisted_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let (first, generated) = load_or_create_token(dir.path(), None).unwrap();
        assert!(generated);
        assert_eq!(first.len(), 64);
        let (second, generated_again) = load_or_create_token(dir.path(), None).unwrap();
        assert_eq!(first, second);
        assert!(!generated_again);
        let (explicit, _) = load_or_create_token(dir.path(), Some("given".into())).unwrap();
        assert_eq!(explicit, "given");
    }
}
