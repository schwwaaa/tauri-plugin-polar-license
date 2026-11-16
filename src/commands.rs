//! Commands callable from the front end as `plugin:polar-license|<name>`.
//! The TypeScript bindings in `guest-js/` wrap these.

use crate::models::{ErrorKind, LaunchCheck, LicenseError, LicenseStatus};
use crate::{messages as msg, PolarLicenseExt};
use tauri::{AppHandle, Runtime};

/// This computer's license state (local file only, instant).
#[tauri::command]
pub(crate) fn status<R: Runtime>(app: AppHandle<R>) -> LicenseStatus {
    app.polar_license().status()
}

/// Activates a key, then starts the app (runs the `on_licensed` hook on the
/// main thread) before returning.
#[tauri::command]
pub(crate) async fn activate<R: Runtime>(
    app: AppHandle<R>,
    key: String,
) -> Result<LicenseStatus, LicenseError> {
    let status = app.polar_license().licensing().activate(&key).await?;
    app.polar_license().set_launch_check(LaunchCheck {
        state: crate::LaunchState::Confirmed,
        message: msg::CHECK_ACTIVATED.into(),
    });
    start_on_main_thread(&app).await?;
    Ok(status)
}

/// Releases this computer's activation. The app decides what happens next
/// (the bundled UI quits the app).
#[tauri::command]
pub(crate) async fn release<R: Runtime>(app: AppHandle<R>) -> Result<(), LicenseError> {
    app.polar_license().licensing().release().await?;
    app.polar_license().set_launch_check(LaunchCheck {
        state: crate::LaunchState::NotLicensed,
        message: msg::CHECK_NOT_LICENSED.into(),
    });
    Ok(())
}

/// Result of this launch's background check (`pending` while it runs).
#[tauri::command]
pub(crate) fn launch_check<R: Runtime>(app: AppHandle<R>) -> LaunchCheck {
    app.polar_license().launch_check()
}

/// Commands run on a background thread; windows and GPU surfaces must be
/// created on the main thread (macOS requires it). Hop there and wait.
async fn start_on_main_thread<R: Runtime>(app: &AppHandle<R>) -> Result<(), LicenseError> {
    let fail = |e: String| {
        LicenseError::new(
            ErrorKind::Unexpected,
            format!("Activated, but the app couldn't start: {e}"),
        )
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(handle.polar_license().run_on_licensed(&handle));
    })
    .map_err(|e| fail(e.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| fail(e.to_string()))?
        .map_err(|_| fail("startup was interrupted".into()))?
        .map_err(fail)
}
