//! The licensing logic, independent of Tauri.
//!
//! [`Licensing`] is what the Tauri plugin uses internally. It can also be
//! used on its own from any Rust app: give it a [`Config`] and the folder to
//! store the license file in.
//!
//! # Lifecycle
//!
//! 1. **Activate (once per computer).** The customer pastes their key. Polar
//!    records an *activation* for this computer and returns its ID. We save
//!    it in `license.json`, bound to this computer's fingerprint.
//! 2. **Every launch.** [`Licensing::status`] reads the file. No network:
//!    the app starts immediately, online or not.
//! 3. **Launch check (background).** [`Licensing::launch_check`] asks Polar
//!    whether the activation is still valid. Only a genuine Polar "not
//!    found" (revoked, disabled, expired, released) marks the file revoked,
//!    which takes effect at the *next* launch. Anything unclear changes
//!    nothing, so licensing can never interrupt a running session.
//! 4. **Release.** [`Licensing::release`] frees this computer's slot so the
//!    key can move to another computer.
//!
//! # "Limit reached" and reclaim
//!
//! Polar refuses a new activation when every slot is taken, even when the
//! activation holding the slot belongs to *this* computer (its license file
//! was lost: app data deleted, app reinstalled). If a reclaim Worker is
//! configured, the plugin asks it to release activations on that key whose
//! `meta.machine` equals this computer's fingerprint, then activates again.
//! A different computer has a different fingerprint and is never released.

use crate::config::Config;
use crate::fingerprint;
use crate::messages as msg;
use crate::models::{ErrorKind, LaunchCheck, LaunchState, LicenseError, LicenseStatus};
use crate::polar::{self, Reply};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// License file format version. Bump only with a migration.
const FILE_VERSION: u32 = 1;

/// What is saved on disk (`license.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLicense {
    version: u32,
    environment: String,
    organization_id: String,
    benefit_id: String,
    key: String,
    display_key: String,
    activation_id: String,
    label: String,
    machine: String,
    activated_at: String,
    /// Set when the launch check finds the activation gone. The record is
    /// kept (not deleted) so the computer still knows its own activation ID.
    #[serde(default)]
    revoked: bool,
}

/// Result of asking the reclaim Worker for help.
#[derive(Debug, PartialEq, Eq)]
enum Reclaim {
    /// This computer's own activation(s) were released: activate again.
    Released,
    /// The key is revoked, disabled or expired.
    KeyInactive,
    /// Nothing of this computer's to release, or no Worker / no answer.
    Nothing,
}

/// The licensing engine for one app.
#[derive(Debug, Clone)]
pub struct Licensing {
    config: Config,
    dir: PathBuf,
    machine: String,
}

impl Licensing {
    /// `dir` is where `license.json` lives (the plugin uses the app's
    /// app-data folder, e.g. `~/Library/Application Support/<identifier>`).
    pub fn new(config: Config, dir: impl Into<PathBuf>) -> Self {
        Self {
            config,
            dir: dir.into(),
            machine: fingerprint::machine_fingerprint(),
        }
    }

    /// Pretend to be a different computer. For tests only.
    #[doc(hidden)]
    pub fn with_machine_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.machine = fingerprint.into();
        self
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Full path of the license file.
    pub fn license_path(&self) -> PathBuf {
        self.dir.join(&self.config.file_name)
    }

    fn support(&self) -> Option<&str> {
        self.config.support_contact.as_deref()
    }

    fn err(&self, kind: ErrorKind, message: &str) -> LicenseError {
        LicenseError::new(kind, msg::with_support(message, self.support()))
    }

    fn log(&self, line: &str) {
        log::debug!(target: "polar_license", "{line}");
        if self.config.log_to_stderr {
            eprintln!("[polar-license] {line}");
        }
    }

    /// API base. Debug builds honor `POLAR_LICENSE_API_BASE_OVERRIDE` so a
    /// local fake Polar can be used; release builds ignore it, so a shipped
    /// app can never be pointed at an "always yes" server.
    fn api_base(&self) -> String {
        #[cfg(debug_assertions)]
        if let Ok(url) = std::env::var("POLAR_LICENSE_API_BASE_OVERRIDE") {
            return url.trim_end_matches('/').to_string();
        }
        self.config.environment.api_base.clone()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_base(), path)
    }

    // ── Storage ─────────────────────────────────────────────────────────

    /// The saved record (active or revoked), only if it was made by this
    /// environment, for this benefit, on this computer.
    fn load_record(&self) -> Option<StoredLicense> {
        let text = fs::read_to_string(self.license_path()).ok()?;
        let s: StoredLicense = serde_json::from_str(&text).ok()?;
        let e = &self.config.environment;
        let ok = s.version == FILE_VERSION
            && s.environment == e.name
            && s.organization_id == e.organization_id
            && s.benefit_id == e.benefit_id
            && s.machine == self.machine;
        ok.then_some(s)
    }

    /// The saved license, only if it's currently active.
    fn load(&self) -> Option<StoredLicense> {
        self.load_record().filter(|s| !s.revoked)
    }

    /// Temp file + rename: a crash can never leave a half-written file.
    fn save(&self, record: &StoredLicense) -> Result<(), LicenseError> {
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(&self.dir)?;
            let tmp = self.dir.join(format!("{}.tmp", self.config.file_name));
            fs::write(
                &tmp,
                serde_json::to_string_pretty(record).expect("serializable"),
            )?;
            fs::rename(&tmp, self.license_path())
        };
        write().map_err(|e| {
            self.log(&format!("couldn't save license file: {e}"));
            self.err(ErrorKind::Storage, msg::STORAGE)
        })
    }

    // ── Status ──────────────────────────────────────────────────────────

    /// This computer's license state. Reads the local file only; instant and
    /// offline-safe.
    pub fn status(&self) -> LicenseStatus {
        let saved = self.load();
        LicenseStatus {
            licensed: saved.is_some(),
            configured: self.config.environment.is_configured(),
            environment: self.config.environment.name.clone(),
            label: saved.as_ref().map(|s| s.label.clone()),
            display_key: saved.as_ref().map(|s| s.display_key.clone()),
            activated_at: saved.as_ref().map(|s| s.activated_at.clone()),
        }
    }

    // ── Activate ────────────────────────────────────────────────────────

    /// Activates `raw_key` on this computer. See the module docs for the
    /// full flow, including reclaim.
    pub async fn activate(&self, raw_key: &str) -> Result<LicenseStatus, LicenseError> {
        let e = &self.config.environment;
        if !e.is_configured() {
            return Err(LicenseError::new(
                ErrorKind::NotConfigured,
                msg::NOT_CONFIGURED,
            ));
        }
        let key = raw_key.trim();
        if key.is_empty() {
            return Err(LicenseError::new(ErrorKind::EmptyKey, msg::EMPTY_KEY));
        }
        if self.load().is_some() {
            return Ok(self.status());
        }

        // This computer had this exact key before and the launch check later
        // marked it revoked. If Polar accepts that activation again (e.g. the
        // key was disabled and re-enabled), reuse it: no new slot needed.
        let previous = self.load_record().filter(|r| r.revoked && r.key == key);
        if let Some(mut rec) = previous.clone() {
            if let Ok(true) = self.activation_is_valid(&rec).await {
                rec.revoked = false;
                self.save(&rec)?;
                self.log("reused this computer's existing activation");
                return Ok(self.status());
            }
        }

        let client = polar::client(self.config.timeout)?;
        let label = fingerprint::machine_label();
        let mut reply = self.request_activation(&client, key, &label).await?;

        // "Limit reached" can be this computer's own old activation (lost
        // license file, or a key revoked then re-enabled: Polar keeps counting
        // the old activation but no longer accepts it). Only the Worker can
        // tell, because only it can see the key's activations.
        if polar::is_limit_reached(&reply) {
            self.log(&format!(
                "activation refused: {} {}",
                reply.status,
                polar::detail_text(&reply.body)
            ));
            match self.reclaim(&client, key).await {
                Reclaim::Released => reply = self.request_activation(&client, key, &label).await?,
                Reclaim::KeyInactive => {
                    return Err(self.err(ErrorKind::Deactivated, msg::DEACTIVATED))
                }
                Reclaim::Nothing => {}
            }
        }

        if reply.status != 200 {
            return Err(self.explain_refusal(&reply, previous.is_some()));
        }

        let activation: polar::ActivationResponse =
            serde_json::from_str(&reply.body).map_err(|e| {
                self.log(&format!("unreadable activation reply: {e}"));
                self.err(ErrorKind::Unexpected, msg::UNEXPECTED)
            })?;

        // A real key, but for another product in the same organization: give
        // the slot straight back so the customer doesn't lose it.
        if activation.license_key.benefit_id != e.benefit_id {
            self.log("key belongs to a different benefit; releasing it again");
            let _ = self.deactivate_remote(&client, key, &activation.id).await;
            return Err(self.err(ErrorKind::WrongProduct, msg::WRONG_PRODUCT));
        }

        self.save(&StoredLicense {
            version: FILE_VERSION,
            environment: e.name.clone(),
            organization_id: e.organization_id.clone(),
            benefit_id: e.benefit_id.clone(),
            key: key.to_string(),
            display_key: activation.license_key.display_key,
            activation_id: activation.id,
            label: activation.label,
            machine: self.machine.clone(),
            activated_at: activation.created_at,
            revoked: false,
        })?;
        self.log("activated");
        Ok(self.status())
    }

    async fn request_activation(
        &self,
        client: &reqwest::Client,
        key: &str,
        label: &str,
    ) -> Result<Reply, LicenseError> {
        let body = serde_json::json!({
            "key": key,
            "organization_id": self.config.environment.organization_id,
            "label": label,
            "meta": { "machine": self.machine, "plugin_version": env!("CARGO_PKG_VERSION") },
        });
        polar::post_json(client, &self.url(polar::ACTIVATE), &body)
            .await
            .map_err(|e| self.network_error(e))
    }

    fn explain_refusal(&self, reply: &Reply, had_revoked_record: bool) -> LicenseError {
        let detail = polar::detail_text(&reply.body);
        self.log(&format!("activation refused: {} {detail}", reply.status));
        match reply.status {
            // No Worker answer, but this computer's own record shows this
            // key was revoked: the best available explanation.
            403 if had_revoked_record => self.err(ErrorKind::Deactivated, msg::DEACTIVATED),
            403 if detail.to_lowercase().contains("limit") => {
                self.err(ErrorKind::InUseElsewhere, msg::IN_USE_ELSEWHERE)
            }
            403 => self.err(ErrorKind::Deactivated, msg::NO_LONGER_ACTIVE),
            404 => {
                let mut text = msg::NOT_RECOGNIZED.to_string();
                if cfg!(debug_assertions) {
                    text.push_str(&format!(
                        " (Test build: using {} Polar.)",
                        self.config.environment.name
                    ));
                }
                self.err(ErrorKind::NotRecognized, &text)
            }
            _ => self.err(ErrorKind::Unexpected, msg::UNEXPECTED),
        }
    }

    fn network_error(&self, e: reqwest::Error) -> LicenseError {
        self.log(&format!("network error: {e}"));
        if e.is_timeout() {
            self.err(ErrorKind::Network, msg::TIMEOUT)
        } else {
            self.err(ErrorKind::Network, msg::NETWORK)
        }
    }

    // ── Validate / launch check ─────────────────────────────────────────

    /// Ok(true) valid · Ok(false) Polar says gone · Err(_) no clear answer.
    async fn activation_is_valid(&self, rec: &StoredLicense) -> Result<bool, String> {
        let client = polar::client(self.config.timeout).map_err(|e| e.message)?;
        let body = serde_json::json!({
            "key": rec.key,
            "organization_id": rec.organization_id,
            "activation_id": rec.activation_id,
            "benefit_id": rec.benefit_id,
        });
        let reply = polar::post_json(&client, &self.url(polar::VALIDATE), &body)
            .await
            .map_err(|e| {
                self.log(&format!("validate network error: {e}"));
                "offline".to_string()
            })?;
        match reply.status {
            200 => Ok(true),
            _ if polar::is_polar_not_found(&reply) => Ok(false),
            other => Err(format!("Polar answered {other}")),
        }
    }

    /// The background check run at launch. Never blocks the app; only a
    /// genuine Polar "not found" changes anything (and only from the next
    /// launch on).
    pub async fn launch_check(&self) -> LaunchCheck {
        let Some(saved) = self.load() else {
            return LaunchCheck {
                state: LaunchState::NotLicensed,
                message: msg::CHECK_NOT_LICENSED.into(),
            };
        };
        match self.activation_is_valid(&saved).await {
            Ok(true) => LaunchCheck {
                state: LaunchState::Confirmed,
                message: msg::CHECK_CONFIRMED.into(),
            },
            Ok(false) => {
                // Only touch the record we checked, not one written since.
                if let Some(mut current) = self
                    .load()
                    .filter(|s| s.activation_id == saved.activation_id)
                {
                    current.revoked = true;
                    let _ = self.save(&current);
                }
                self.log("launch check: activation no longer valid; gate appears next launch");
                LaunchCheck {
                    state: LaunchState::Deactivated,
                    message: msg::with_support(msg::CHECK_DEACTIVATED, self.support()),
                }
            }
            Err(why) => {
                self.log(&format!("launch check inconclusive: {why}"));
                LaunchCheck {
                    state: LaunchState::Offline,
                    message: msg::CHECK_OFFLINE.into(),
                }
            }
        }
    }

    // ── Release ─────────────────────────────────────────────────────────

    /// "Release this machine": frees this computer's slot on Polar, then
    /// deletes the local file. The file is deleted only after Polar
    /// confirms, so a failed call never strands the slot.
    pub async fn release(&self) -> Result<(), LicenseError> {
        let saved = self.load().ok_or_else(|| {
            LicenseError::new(ErrorKind::NothingToRelease, msg::NOTHING_TO_RELEASE)
        })?;
        let client = polar::client(self.config.timeout)?;
        self.deactivate_remote(&client, &saved.key, &saved.activation_id)
            .await?;
        fs::remove_file(self.license_path()).map_err(|e| {
            self.log(&format!(
                "released on Polar, but couldn't delete the license file: {e}"
            ));
            self.err(ErrorKind::Storage, msg::STORAGE)
        })?;
        self.log("released");
        Ok(())
    }

    async fn deactivate_remote(
        &self,
        client: &reqwest::Client,
        key: &str,
        activation_id: &str,
    ) -> Result<(), LicenseError> {
        let body = serde_json::json!({
            "key": key,
            "organization_id": self.config.environment.organization_id,
            "activation_id": activation_id,
        });
        let reply = polar::post_json(client, &self.url(polar::DEACTIVATE), &body)
            .await
            .map_err(|e| self.network_error(e))?;
        match reply.status {
            204 => Ok(()),
            // Already gone (e.g. released from the dashboard): that's the goal.
            _ if polar::is_polar_not_found(&reply) => Ok(()),
            other => {
                self.log(&format!(
                    "release refused: {other} {}",
                    polar::detail_text(&reply.body)
                ));
                Err(self.err(ErrorKind::Unexpected, msg::RELEASE_FAILED))
            }
        }
    }

    // ── Reclaim ─────────────────────────────────────────────────────────

    fn reclaim_base(&self) -> Option<String> {
        let url = self.config.reclaim_url.as_deref()?.trim();
        (!url.is_empty() && !url.to_ascii_uppercase().starts_with("PASTE"))
            .then(|| url.trim_end_matches('/').to_string())
    }

    async fn reclaim(&self, client: &reqwest::Client, key: &str) -> Reclaim {
        let Some(base) = self.reclaim_base() else {
            self.log("reclaim: off (no reclaim_url configured)");
            return Reclaim::Nothing;
        };
        let body = serde_json::json!({ "key": key, "machine": self.machine });
        let Ok(reply) = polar::post_json(client, &format!("{base}/api/reclaim"), &body).await
        else {
            self.log("reclaim: Worker unreachable");
            return Reclaim::Nothing;
        };
        self.log(&format!("reclaim: {}", reply.body));
        let state = serde_json::from_str::<serde_json::Value>(&reply.body)
            .ok()
            .and_then(|v| v.get("state").and_then(|s| s.as_str()).map(str::to_string));
        match state.as_deref() {
            Some("reclaimed") => Reclaim::Released,
            Some("inactive") => Reclaim::KeyInactive,
            _ => Reclaim::Nothing,
        }
    }
}
