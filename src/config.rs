//! Configuration: which Polar to talk to, and how the plugin behaves.

use std::time::Duration;

/// Base URL of Polar's production API.
pub const PRODUCTION_API: &str = "https://api.polar.sh";
/// Base URL of Polar's sandbox API.
pub const SANDBOX_API: &str = "https://sandbox-api.polar.sh";

/// One Polar world: production or sandbox.
///
/// Polar's sandbox and production are completely separate: different
/// organizations, products, benefit IDs and keys. A key issued in one does
/// not exist in the other, so each environment carries its own IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// `"production"` or `"sandbox"`. Saved in the license file, so a sandbox
    /// activation can never unlock a production build (or the reverse).
    pub name: String,
    /// API base URL, without a trailing slash.
    pub api_base: String,
    /// Your Polar organization ID (Settings → General).
    pub organization_id: String,
    /// The ID of this app's **License Keys benefit**. Keys are accepted only
    /// if they belong to this benefit, so a key bought for one of your apps
    /// can't unlock another one in the same organization.
    pub benefit_id: String,
}

impl Environment {
    /// Real Polar (`api.polar.sh`).
    pub fn production(organization_id: impl Into<String>, benefit_id: impl Into<String>) -> Self {
        Self::custom("production", PRODUCTION_API, organization_id, benefit_id)
    }

    /// Polar's sandbox (`sandbox-api.polar.sh`): test cards, fake money.
    pub fn sandbox(organization_id: impl Into<String>, benefit_id: impl Into<String>) -> Self {
        Self::custom("sandbox", SANDBOX_API, organization_id, benefit_id)
    }

    /// Any API base, e.g. a local fake Polar in tests.
    pub fn custom(
        name: impl Into<String>,
        api_base: impl Into<String>,
        organization_id: impl Into<String>,
        benefit_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            api_base: api_base.into().trim_end_matches('/').to_string(),
            organization_id: organization_id.into().trim().to_string(),
            benefit_id: benefit_id.into().trim().to_string(),
        }
    }

    /// `false` while either ID is empty or still a `PASTE-…` placeholder.
    pub fn is_configured(&self) -> bool {
        let real = |s: &str| !s.is_empty() && !s.to_ascii_uppercase().starts_with("PASTE");
        real(&self.organization_id) && real(&self.benefit_id)
    }
}

/// Everything the licensing core needs. Built for you by [`crate::Builder`];
/// construct it directly only when using [`crate::Licensing`] without Tauri.
#[derive(Debug, Clone)]
pub struct Config {
    /// The Polar environment this build talks to.
    pub environment: Environment,
    /// Address of your reclaim Worker (see `worker/`), e.g.
    /// `https://polar-license.you.workers.dev`. `None` turns reclaim off.
    pub reclaim_url: Option<String>,
    /// Shown to customers at the end of license messages:
    /// "Need help? Contact `<this>`." `None` leaves the line out.
    pub support_contact: Option<String>,
    /// Print `[polar-license] …` lines to stderr (useful in development).
    /// Everything is also sent to the `log` crate at debug/warn level.
    pub log_to_stderr: bool,
    /// File name of the saved license inside the app-data folder.
    pub file_name: String,
    /// Timeout for every network call.
    pub timeout: Duration,
}

impl Config {
    /// Defaults: no reclaim, no support line, stderr logging in debug
    /// builds only, `license.json`, 15-second timeout.
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            reclaim_url: None,
            support_contact: None,
            log_to_stderr: cfg!(debug_assertions),
            file_name: "license.json".into(),
            timeout: Duration::from_secs(15),
        }
    }
}
