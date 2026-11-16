//! # tauri-plugin-polar-license
//!
//! [Polar.sh](https://polar.sh) license keys for Tauri v2 desktop apps.
//!
//! - **One-time activation** per computer, with Polar's activation limits.
//! - **Never blocks a running app.** Launches read a local file and start
//!   instantly, online or offline; a background check catches revoked keys
//!   and applies them at the next launch.
//! - **Release this machine** so customers can move their key themselves.
//! - **Automatic recovery** (optional): a computer that lost its license
//!   file gets its own activation back, via a small free Worker.
//! - **Startup gating:** your main window or engine starts only once the
//!   license is valid ([`Builder::on_licensed`]).
//! - Works on macOS, Windows and Linux.
//!
//! ```rust,ignore
//! fn main() {
//!     tauri::Builder::default()
//!         .plugin(
//!             tauri_plugin_polar_license::Builder::new()
//!                 .production("POLAR-ORG-ID", "LICENSE-KEYS-BENEFIT-ID")
//!                 .sandbox("SANDBOX-ORG-ID", "SANDBOX-BENEFIT-ID")
//!                 .use_sandbox(cfg!(debug_assertions))
//!                 .support_contact("support@example.com")
//!                 .on_licensed(|app| start_main_window(app))
//!                 .build(),
//!         )
//!         .run(tauri::generate_context!())
//!         .expect("error while running app");
//! }
//! ```
//!
//! See the README and `docs/` for setup in Polar, the front-end API, the
//! drop-in UI and the reclaim Worker.

#![forbid(unsafe_code)]

mod commands;
pub mod config;
pub mod core;
pub mod fingerprint;
mod messages;
pub mod models;
mod polar;

pub use crate::config::{Config, Environment, PRODUCTION_API, SANDBOX_API};
pub use crate::core::Licensing;
pub use crate::models::{ErrorKind, LaunchCheck, LaunchState, LicenseError, LicenseStatus};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::plugin::{Builder as PluginBuilder, TauriPlugin};
use tauri::{AppHandle, Manager, RunEvent, Runtime};

type Hook<R> = Arc<dyn Fn(&AppHandle<R>) -> Result<(), String> + Send + Sync + 'static>;

/// Plugin state, available anywhere you have an `AppHandle`/`App`/`Window`
/// through [`PolarLicenseExt::polar_license`].
pub struct PolarLicense<R: Runtime> {
    licensing: Licensing,
    on_licensed: Option<Hook<R>>,
    started: AtomicBool,
    launch_check: Mutex<LaunchCheck>,
}

impl<R: Runtime> PolarLicense<R> {
    /// This computer's license state (local file, instant).
    pub fn status(&self) -> LicenseStatus {
        self.licensing.status()
    }

    /// Shorthand for `status().licensed`.
    pub fn is_licensed(&self) -> bool {
        self.status().licensed
    }

    /// Result of this launch's background check.
    pub fn launch_check(&self) -> LaunchCheck {
        self.launch_check
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }

    /// The underlying engine (activate / release / launch check).
    pub fn licensing(&self) -> &Licensing {
        &self.licensing
    }

    fn set_launch_check(&self, value: LaunchCheck) {
        if let Ok(mut slot) = self.launch_check.lock() {
            *slot = value;
        }
    }

    /// Runs the `on_licensed` hook at most once per process.
    fn run_on_licensed(&self, app: &AppHandle<R>) -> Result<(), String> {
        if self.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        match &self.on_licensed {
            Some(hook) => hook(app),
            None => Ok(()),
        }
    }
}

/// Access the plugin from Rust: `app.polar_license().is_licensed()`.
pub trait PolarLicenseExt<R: Runtime> {
    fn polar_license(&self) -> &PolarLicense<R>;
}

impl<R: Runtime, T: Manager<R>> PolarLicenseExt<R> for T {
    fn polar_license(&self) -> &PolarLicense<R> {
        self.state::<PolarLicense<R>>().inner()
    }
}

/// Configures and builds the plugin.
pub struct Builder<R: Runtime> {
    production: Option<Environment>,
    sandbox: Option<Environment>,
    use_sandbox: bool,
    reclaim_url: Option<String>,
    support_contact: Option<String>,
    log_to_stderr: bool,
    file_name: String,
    on_licensed: Option<Hook<R>>,
}

impl<R: Runtime> Default for Builder<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Runtime> Builder<R> {
    pub fn new() -> Self {
        Self {
            production: None,
            sandbox: None,
            use_sandbox: false,
            reclaim_url: None,
            support_contact: None,
            log_to_stderr: cfg!(debug_assertions),
            file_name: "license.json".into(),
            on_licensed: None,
        }
    }

    /// Your **production** Polar organization ID and License Keys benefit ID.
    pub fn production(
        mut self,
        organization_id: impl Into<String>,
        benefit_id: impl Into<String>,
    ) -> Self {
        self.production = Some(Environment::production(organization_id, benefit_id));
        self
    }

    /// Your **sandbox** organization ID and benefit ID (different from
    /// production's: the two Polar environments share nothing).
    pub fn sandbox(
        mut self,
        organization_id: impl Into<String>,
        benefit_id: impl Into<String>,
    ) -> Self {
        self.sandbox = Some(Environment::sandbox(organization_id, benefit_id));
        self
    }

    /// Talk to the sandbox instead of production. Typical:
    /// `.use_sandbox(cfg!(debug_assertions))`. Ignored if no sandbox IDs set.
    pub fn use_sandbox(mut self, yes: bool) -> Self {
        self.use_sandbox = yes;
        self
    }

    /// Use any environment (e.g. a local fake Polar).
    pub fn environment(mut self, environment: Environment) -> Self {
        self.production = Some(environment);
        self.use_sandbox = false;
        self
    }

    /// Address of your reclaim Worker (see `worker/`). Enables automatic
    /// recovery when a computer's license file is lost.
    pub fn reclaim_url(mut self, url: impl Into<String>) -> Self {
        self.reclaim_url = Some(url.into());
        self
    }

    /// Shown to customers as "Need help? Contact `<this>`."
    pub fn support_contact(mut self, contact: impl Into<String>) -> Self {
        self.support_contact = Some(contact.into());
        self
    }

    /// Print `[polar-license] …` lines to stderr. Default: debug builds only.
    pub fn log_to_stderr(mut self, yes: bool) -> Self {
        self.log_to_stderr = yes;
        self
    }

    /// License file name in the app-data folder. Default `license.json`.
    /// **Never change this after shipping**: existing customers' files
    /// would no longer be found.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = name.into();
        self
    }

    /// Called once the license is valid: at startup when this computer is
    /// already licensed, or right after a successful activation. Runs on the
    /// main thread, at most once per process. Start your main window,
    /// renderer or engine here.
    pub fn on_licensed<F>(mut self, hook: F) -> Self
    where
        F: Fn(&AppHandle<R>) -> Result<(), String> + Send + Sync + 'static,
    {
        self.on_licensed = Some(Arc::new(hook));
        self
    }

    fn config(&self) -> Config {
        let environment = match (&self.production, &self.sandbox) {
            (_, Some(sandbox)) if self.use_sandbox => sandbox.clone(),
            (Some(production), _) => production.clone(),
            (None, Some(sandbox)) => sandbox.clone(),
            (None, None) => Environment::production("", ""),
        };
        let mut config = Config::new(environment);
        config.reclaim_url = self.reclaim_url.clone();
        config.support_contact = self.support_contact.clone();
        config.log_to_stderr = self.log_to_stderr;
        config.file_name = self.file_name.clone();
        config
    }

    pub fn build(self) -> TauriPlugin<R> {
        let config = self.config();
        let on_licensed = self.on_licensed;
        PluginBuilder::new("polar-license")
            .invoke_handler(tauri::generate_handler![
                commands::status,
                commands::activate,
                commands::release,
                commands::launch_check,
            ])
            .setup(move |app, _api| {
                let dir = app.path().app_data_dir()?;
                let licensing = Licensing::new(config, dir);
                let licensed = licensing.status().licensed;
                let initial = if licensed {
                    LaunchCheck {
                        state: LaunchState::Pending,
                        message: messages::CHECK_PENDING.into(),
                    }
                } else {
                    LaunchCheck {
                        state: LaunchState::NotLicensed,
                        message: messages::CHECK_NOT_LICENSED.into(),
                    }
                };
                app.manage(PolarLicense::<R> {
                    licensing,
                    on_licensed,
                    started: AtomicBool::new(false),
                    launch_check: Mutex::new(initial),
                });
                if licensed {
                    let handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = handle.polar_license().licensing().launch_check().await;
                        handle.polar_license().set_launch_check(result);
                    });
                }
                Ok(())
            })
            .on_event(|app, event| {
                // Ready = after the app's own setup, on the main thread.
                if let RunEvent::Ready = event {
                    let state = app.polar_license();
                    if state.is_licensed() {
                        if let Err(e) = state.run_on_licensed(app) {
                            log::error!(target: "polar_license", "on_licensed failed: {e}");
                            eprintln!("[polar-license] on_licensed failed: {e}");
                        }
                    }
                }
            })
            .build()
    }
}
