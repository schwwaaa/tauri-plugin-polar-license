//! Types shared with the app's front end (serialized as camelCase JSON).

use serde::{Deserialize, Serialize};

/// The license state of this computer, as the app sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    /// `true` when a valid, non-revoked license for this environment,
    /// benefit and computer is saved.
    pub licensed: bool,
    /// `false` while the Polar IDs are missing or placeholders.
    pub configured: bool,
    /// `"production"` or `"sandbox"`.
    pub environment: String,
    /// This computer's name as recorded with the activation in Polar.
    pub label: Option<String>,
    /// Masked key, e.g. `****-E304DA`.
    pub display_key: Option<String>,
    /// When Polar created this activation (ISO 8601).
    pub activated_at: Option<String>,
}

/// Outcome of the background check that runs at each launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchState {
    /// Still running.
    Pending,
    /// Nothing to check: this computer isn't activated.
    NotLicensed,
    /// Polar confirmed the activation (or it was just activated).
    Confirmed,
    /// Polar says the key or activation is no longer valid (revoked,
    /// disabled, expired or released). The current session keeps running;
    /// the activation gate appears at the next launch.
    Deactivated,
    /// No clear answer (offline, timeout, Polar outage). Nothing changed.
    Offline,
}

/// [`LaunchState`] plus a sentence suitable for showing to the customer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchCheck {
    pub state: LaunchState,
    pub message: String,
}

/// What went wrong, so the app can react per case or show its own wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// The Polar IDs for this build are missing or placeholders.
    NotConfigured,
    /// The customer submitted an empty key.
    EmptyKey,
    /// Polar doesn't know this key (typo, or a key from the other environment).
    NotRecognized,
    /// The key is real but belongs to a different benefit (another product).
    WrongProduct,
    /// Every activation slot is taken by other computers.
    InUseElsewhere,
    /// The key is revoked, disabled or expired.
    Deactivated,
    /// No connection to Polar.
    Network,
    /// The license file couldn't be read or written.
    Storage,
    /// "Release" was requested but this computer isn't activated.
    NothingToRelease,
    /// Anything else (the developer log has Polar's raw reply).
    Unexpected,
}

/// An error with a customer-ready message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct LicenseError {
    pub kind: ErrorKind,
    pub message: String,
}

impl LicenseError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
