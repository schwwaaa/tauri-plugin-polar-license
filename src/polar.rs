//! The four HTTP calls the plugin makes, and the parts of Polar's replies it
//! reads.
//!
//! Polar's customer-portal license endpoints are **public**: they need no
//! access token, which is what makes it safe to call them from a shipped
//! app. (Listing a key's activations is *not* public; that's why reclaim
//! goes through your own Worker.)
//!
//! | Call | Endpoint |
//! |---|---|
//! | activate   | `POST /v1/customer-portal/license-keys/activate`   |
//! | validate   | `POST /v1/customer-portal/license-keys/validate`   |
//! | deactivate | `POST /v1/customer-portal/license-keys/deactivate` |
//! | reclaim    | `POST <your Worker>/api/reclaim`                   |

use crate::models::{ErrorKind, LicenseError};
use serde::Deserialize;
use std::time::Duration;

pub(crate) const ACTIVATE: &str = "/v1/customer-portal/license-keys/activate";
pub(crate) const VALIDATE: &str = "/v1/customer-portal/license-keys/validate";
pub(crate) const DEACTIVATE: &str = "/v1/customer-portal/license-keys/deactivate";

/// HTTP status + raw body. Kept raw so the exact reply can be logged.
pub(crate) struct Reply {
    pub status: u16,
    pub body: String,
}

pub(crate) fn client(timeout: Duration) -> Result<reqwest::Client, LicenseError> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(concat!(
            "tauri-plugin-polar-license/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| LicenseError::new(ErrorKind::Unexpected, format!("HTTP client error: {e}")))
}

pub(crate) async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<Reply, reqwest::Error> {
    let response = client.post(url).json(body).send().await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    Ok(Reply { status, body })
}

/// The fields of a successful activation that the plugin uses.
#[derive(Debug, Deserialize)]
pub(crate) struct ActivationResponse {
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub license_key: GrantedKey,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GrantedKey {
    pub benefit_id: String,
    pub display_key: String,
}

#[derive(Debug, Deserialize)]
struct PolarError {
    #[serde(default)]
    error: String,
    #[serde(default)]
    detail: serde_json::Value,
}

/// Polar's human-readable reason, for the developer log.
pub(crate) fn detail_text(body: &str) -> String {
    match serde_json::from_str::<PolarError>(body).map(|p| p.detail) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("msg").and_then(|m| m.as_str()))
            .collect::<Vec<_>>()
            .join("; "),
        _ => body.chars().take(200).collect(),
    }
}

/// `true` only for a genuine Polar "not found" (a JSON body whose `error` is
/// `ResourceNotFound`). A 404 from anything else — a captive Wi-Fi portal, a
/// proxy — must never be taken as "this license is gone".
pub(crate) fn is_polar_not_found(reply: &Reply) -> bool {
    reply.status == 404
        && serde_json::from_str::<PolarError>(&reply.body)
            .map(|p| p.error == "ResourceNotFound")
            .unwrap_or(false)
}

/// Polar answers 403 with "activation limit reached" when every slot is
/// taken — including by this computer's own earlier activation.
pub(crate) fn is_limit_reached(reply: &Reply) -> bool {
    reply.status == 403 && detail_text(&reply.body).to_lowercase().contains("limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(status: u16, body: &str) -> Reply {
        Reply {
            status,
            body: body.into(),
        }
    }

    #[test]
    fn only_real_polar_404s_count() {
        assert!(is_polar_not_found(&reply(
            404,
            r#"{"error":"ResourceNotFound","detail":"Not found"}"#
        )));
        assert!(!is_polar_not_found(&reply(
            404,
            "<html>Hotel Wi-Fi login</html>"
        )));
        assert!(!is_polar_not_found(&reply(
            404,
            r#"{"detail":"unknown path"}"#
        )));
    }

    #[test]
    fn limit_reached_detection() {
        assert!(is_limit_reached(&reply(
            403,
            r#"{"error":"NotPermitted","detail":"License key activation limit already reached"}"#
        )));
        assert!(!is_limit_reached(&reply(
            403,
            r#"{"error":"NotPermitted","detail":"License key is revoked"}"#
        )));
    }

    #[test]
    fn validation_errors_are_flattened() {
        let body = r#"{"detail":[{"loc":["body","organization_id"],"msg":"Input should be a valid UUID"}]}"#;
        assert_eq!(detail_text(body), "Input should be a valid UUID");
    }
}
