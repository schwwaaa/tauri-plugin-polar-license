//! Every sentence a customer can see, in one place.
//!
//! Rules: plain language, only mention things that exist in the app, never
//! show Polar's raw replies (those go to the developer log).

/// Appends "Need help? Contact …." when a support contact is configured.
pub(crate) fn with_support(message: &str, support: Option<&str>) -> String {
    match support {
        Some(contact) if !contact.trim().is_empty() => {
            format!("{message} Need help? Contact {}.", contact.trim())
        }
        _ => message.to_string(),
    }
}

pub(crate) const NOT_CONFIGURED: &str =
    "This build has no Polar IDs for its environment yet. Add them to the plugin's configuration and rebuild.";
pub(crate) const EMPTY_KEY: &str = "Paste your license key first.";
pub(crate) const NOT_RECOGNIZED: &str =
    "That license key wasn't recognized. Check that you pasted the whole key.";
pub(crate) const WRONG_PRODUCT: &str =
    "This license key is for a different product. Check that you're using the key from this app's purchase.";
pub(crate) const IN_USE_ELSEWHERE: &str =
    "This license key is already activated on another computer. Each key works on a limited number of computers at a time. To move it here, open the app on the other computer and click \"Release this machine\" in the License section, then try again.";
pub(crate) const DEACTIVATED: &str = "This license key has been deactivated.";
pub(crate) const NO_LONGER_ACTIVE: &str = "This license key is no longer active.";
pub(crate) const UNEXPECTED: &str = "Activation didn't work. Try again in a moment.";
pub(crate) const NETWORK: &str =
    "Couldn't connect to the license server. Check your internet connection and try again.";
pub(crate) const TIMEOUT: &str =
    "The license server didn't answer. Check your internet connection and try again.";
pub(crate) const NOTHING_TO_RELEASE: &str = "This computer has no active license to release.";
pub(crate) const RELEASE_FAILED: &str =
    "Couldn't release this computer right now. Try again in a moment.";
pub(crate) const STORAGE: &str =
    "The license couldn't be saved on this computer. Check that the app can write to its data folder.";

// Launch check (shown on the license panel).
pub(crate) const CHECK_CONFIRMED: &str = "License confirmed.";
pub(crate) const CHECK_ACTIVATED: &str = "Activated with Polar.";
pub(crate) const CHECK_PENDING: &str = "Checking your license…";
pub(crate) const CHECK_NOT_LICENSED: &str = "Not activated.";
pub(crate) const CHECK_OFFLINE: &str =
    "License not checked this time (no connection). Nothing changed.";
pub(crate) const CHECK_DEACTIVATED: &str =
    "This license has been deactivated. The app keeps working until you close it; next time it opens, it will ask for a license key.";
