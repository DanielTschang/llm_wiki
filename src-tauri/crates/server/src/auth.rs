//! Single-owner access control.
//!
//! Browsers sign in once with the token and get an `HttpOnly`,
//! `SameSite=Strict` session cookie; scripts send `Authorization: Bearer`.
//! Cookie-authenticated state-changing requests must also carry
//! [`CLIENT_HEADER`]: a cross-site page cannot add a custom header without a
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
    None,
}

pub struct Auth {
    token_digest: [u8; 32],
    session_value: String,
    secure_cookie: bool,
}

impl Auth {
    pub fn new(token: &str, secure_cookie: bool) -> Self {
        let token_digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let session_value = hex(&Sha256::digest(
            [b"llm-wiki-session:".as_slice(), token.as_bytes()].concat(),
        ));
        Self {
            token_digest,
            session_value,
            secure_cookie,
        }
    }

    pub fn token_matches(&self, candidate: &str) -> bool {
        let digest: [u8; 32] = Sha256::digest(candidate.as_bytes()).into();
        digest.ct_eq(&self.token_digest).into()
    }

    pub fn credential(&self, headers: &HeaderMap) -> Credential {
        if let Some(bearer) = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
        {
            if self.token_matches(bearer.trim()) {
                return Credential::Bearer;
            }
        }
        let cookie_ok = headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .any(|(name, value)| {
                name == COOKIE_NAME
                    && bool::from(value.as_bytes().ct_eq(self.session_value.as_bytes()))
            });
        if cookie_ok {
            Credential::Cookie
        } else {
            Credential::None
        }
    }

    pub fn session_cookie(&self) -> String {
        format!(
            "{COOKIE_NAME}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000{}",
            self.session_value,
            if self.secure_cookie { "; Secure" } else { "" }
        )
    }

    pub fn clear_cookie(&self) -> String {
        format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
    }
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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
    fn accepts_bearer_token_and_session_cookie_only() {
        let auth = Auth::new("s3cret", false);
        let cookie = auth.session_cookie();
        let cookie_pair = cookie.split(';').next().unwrap();

        assert_eq!(
            auth.credential(&headers(&[(header::AUTHORIZATION, "Bearer s3cret")])),
            Credential::Bearer
        );
        assert_eq!(
            auth.credential(&headers(&[(
                header::COOKIE,
                &format!("a=b; {cookie_pair}")
            )])),
            Credential::Cookie
        );
        assert_eq!(
            auth.credential(&headers(&[(header::AUTHORIZATION, "Bearer wrong")])),
            Credential::None
        );
        assert_eq!(
            auth.credential(&headers(&[(header::COOKIE, "llm_wiki_session=s3cret")])),
            Credential::None,
            "the raw token is not a valid session value"
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
