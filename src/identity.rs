use base64::Engine;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountIdentity {
    pub email: String,
    pub name: String,
    pub plan: String,
    pub account_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthFile {
    #[serde(default)]
    tokens: Option<Tokens>,
    #[serde(default, rename = "OPENAI_API_KEY")]
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Tokens {
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub identity: AccountIdentity,
    pub access_token: Option<String>,
    pub account_id: Option<String>,
}

pub fn parse_auth_bytes(bytes: &[u8]) -> AppResult<AuthCredentials> {
    let auth: AuthFile = serde_json::from_slice(bytes)?;
    let has_api_key = auth
        .api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let tokens = auth.tokens.unwrap_or(Tokens {
        id_token: None,
        account_id: None,
        access_token: None,
        refresh_token: None,
    });

    let has_oauth_credential = [
        tokens.id_token.as_deref(),
        tokens.access_token.as_deref(),
        tokens.refresh_token.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty());
    if !has_api_key && !has_oauth_credential {
        return Err(AppError::msg(
            "auth.json 不包含可用的 OAuth token 或 OPENAI_API_KEY",
        ));
    }

    if let Some(account_id) = tokens.account_id.as_deref() {
        if account_id.len() > 256 || !account_id.is_ascii() {
            return Err(AppError::msg(
                "account_id 必须是长度不超过 256 的 ASCII 文本",
            ));
        }
    }

    let mut identity = AccountIdentity {
        account_id: tokens.account_id.clone().unwrap_or_default(),
        ..AccountIdentity::default()
    };

    if let Some(id_token) = tokens.id_token.as_deref() {
        if let Some(claims) = decode_jwt_claims(id_token) {
            identity.email = claims
                .get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            identity.name = claims
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(openai_auth) = claims.get("https://api.openai.com/auth") {
                if let Some(plan) = openai_auth
                    .get("chatgpt_plan_type")
                    .and_then(|v| v.as_str())
                {
                    identity.plan = plan.to_string();
                }
                if identity.account_id.is_empty() {
                    if let Some(aid) = openai_auth
                        .get("chatgpt_account_id")
                        .and_then(|v| v.as_str())
                    {
                        identity.account_id = aid.to_string();
                    }
                }
            }
        }
    }

    Ok(AuthCredentials {
        identity,
        access_token: tokens.access_token.filter(|s| !s.is_empty()),
        account_id: tokens.account_id.filter(|s| !s.is_empty()),
    })
}

/// Confirm that live credentials still belong to the profile selected by the
/// current marker. Unknown identities only match when the full payload is
/// unchanged; this deliberately fails closed after an external login.
pub fn same_account(left: &[u8], right: &[u8]) -> AppResult<bool> {
    let left_key = account_key(left)?;
    let right_key = account_key(right)?;
    Ok(match (left_key, right_key) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    })
}

fn account_key(bytes: &[u8]) -> AppResult<Option<String>> {
    let auth: AuthFile = serde_json::from_slice(bytes)?;
    if let Some(tokens) = auth.tokens.as_ref() {
        if let Some(account_id) = tokens
            .account_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if account_id.len() > 256 || !account_id.is_ascii() {
                return Err(AppError::msg(
                    "account_id 必须是长度不超过 256 的 ASCII 文本",
                ));
            }
            return Ok(Some(format!("account:{account_id}")));
        }
        if let Some(email) = tokens
            .id_token
            .as_deref()
            .and_then(decode_jwt_claims)
            .and_then(|claims| {
                claims
                    .get("email")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(format!("email:{email}")));
        }
    }
    Ok(auth
        .api_key
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("api:{value}")))
}

pub fn decode_jwt_claims(token: &str) -> Option<Value> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let decoded = engine
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    serde_json::from_slice(&decoded).ok()
}

pub fn short_account_id(account_id: &str) -> String {
    let chars: Vec<char> = account_id.chars().collect();
    if chars.len() <= 12 {
        return account_id.to_string();
    }
    let prefix: String = chars.iter().take(8).collect();
    let suffix: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn make_id_token(email: &str, plan: &str, account_id: &str) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = serde_json::json!({
            "email": email,
            "name": "Test User",
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": plan,
                "chatgpt_account_id": account_id,
            }
        });
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        format!("{header}.{payload_b64}.sig")
    }

    #[test]
    fn parses_identity_from_auth_json() {
        let id_token = make_id_token("a@example.com", "pro", "acc-1234567890");
        let auth = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "id_token": id_token,
                "access_token": "secret-access",
                "account_id": "acc-1234567890",
                "refresh_token": "secret-refresh"
            }
        });
        let creds = parse_auth_bytes(serde_json::to_vec(&auth).unwrap().as_slice()).unwrap();
        assert_eq!(creds.identity.email, "a@example.com");
        assert_eq!(creds.identity.plan, "pro");
        assert_eq!(creds.identity.account_id, "acc-1234567890");
        assert_eq!(creds.access_token.as_deref(), Some("secret-access"));
    }

    #[test]
    fn rejects_payload_without_credentials() {
        assert!(parse_auth_bytes(br#"{}"#).is_err());
        assert!(parse_auth_bytes(br#"{"tokens":{"account_id":"only-id"}}"#).is_err());
    }

    #[test]
    fn supports_api_key_and_compares_accounts() {
        let a = br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-test-a"}"#;
        let a_copy = br#"{"OPENAI_API_KEY":"sk-test-a","auth_mode":"apikey"}"#;
        let b = br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-test-b"}"#;
        assert!(parse_auth_bytes(a).is_ok());
        assert!(same_account(a, a_copy).unwrap());
        assert!(!same_account(a, b).unwrap());
    }

    #[test]
    fn short_account_id_is_unicode_safe() {
        assert_eq!(
            short_account_id("账号账号账号账号账号账号账号"),
            "账号账号账号账号…账号账号"
        );
    }
}
