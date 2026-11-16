# Rust API

## `Builder`

```rust
use tauri_plugin_polar_license::Builder;

tauri::Builder::default()
    .plugin(
        Builder::new()
            .production("PROD-ORG-ID", "PROD-BENEFIT-ID")
            .sandbox("SANDBOX-ORG-ID", "SANDBOX-BENEFIT-ID")
            .use_sandbox(cfg!(debug_assertions))
            .reclaim_url("https://polar-license.you.workers.dev")
            .support_contact("support@yourapp.com")
            .on_licensed(|app| start_app(app))
            .build(),
    )
```

| Method | Default | Meaning |
|---|---|---|
| `.production(org, benefit)` | — | Production organization ID and License Keys benefit ID |
| `.sandbox(org, benefit)` | — | Sandbox IDs (the sandbox shares nothing with production) |
| `.use_sandbox(bool)` | `false` | Use the sandbox IDs. Typical: `cfg!(debug_assertions)`. Ignored without sandbox IDs. |
| `.environment(Environment)` | — | Any environment, e.g. `Environment::custom(…)` for a local fake Polar |
| `.reclaim_url(url)` | off | Your [reclaim Worker](worker.md); enables automatic recovery |
| `.support_contact(text)` | none | Appended to customer messages: "Need help? Contact …." |
| `.log_to_stderr(bool)` | debug builds | Print `[polar-license] …` lines (everything also goes to the `log` crate) |
| `.file_name(name)` | `license.json` | License file name in the app-data folder. **Never change after shipping.** |
| `.on_licensed(Fn(&AppHandle) -> Result<(), String>)` | none | Start your app (see below) |

**Which environment is used:** with `use_sandbox(true)` and sandbox IDs set, the sandbox; otherwise production; if only sandbox IDs are set, the sandbox. With no IDs, `status().configured` is `false` and activation reports `notConfigured`.

### `on_licensed`: startup gating

```rust
fn start_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
        .title("My App")
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

- Runs **at most once per process**, **on the main thread**. Safe for creating windows and GPU surfaces; macOS requires the main thread for both.
- Called at `RunEvent::Ready` when the computer is already licensed, after your own `.setup()` has run. Or it's called inside `activate`, before the front end's `activate()` promise resolves.
- An `Err` from the hook is logged at startup, and at activation it's returned to the front end as an `unexpected` error.

Pattern: keep a lightweight window (or just the gate) open at startup, and
open your heavy window/renderer/engine in `on_licensed`.

## `PolarLicenseExt`

Available on `App`, `AppHandle`, `Window` and anything else that implements `Manager`:

```rust
use tauri_plugin_polar_license::PolarLicenseExt;

if app.polar_license().is_licensed() { /* … */ }
let status = app.polar_license().status();        // LicenseStatus
let check  = app.polar_license().launch_check();  // LaunchCheck
```

## Types

All front-end types serialize as camelCase JSON.

```rust
pub struct LicenseStatus {
    pub licensed: bool,              // valid license for this environment, benefit and computer
    pub configured: bool,            // Polar IDs present
    pub environment: String,         // "production" | "sandbox"
    pub label: Option<String>,       // computer name recorded in Polar
    pub display_key: Option<String>, // "****-E304DA"
    pub activated_at: Option<String>,
}

pub enum LaunchState { Pending, NotLicensed, Confirmed, Deactivated, Offline }
pub struct LaunchCheck { pub state: LaunchState, pub message: String }

pub enum ErrorKind {
    NotConfigured, EmptyKey, NotRecognized, WrongProduct, InUseElsewhere,
    Deactivated, Network, Storage, NothingToRelease, Unexpected,
}
pub struct LicenseError { pub kind: ErrorKind, pub message: String } // message is customer-ready
```

## Using the core without Tauri

`Licensing` has no Tauri dependency in its API. Use it from any Rust program:

```rust
use tauri_plugin_polar_license::{Config, Environment, Licensing};

let mut config = Config::new(Environment::production("ORG-ID", "BENEFIT-ID"));
config.support_contact = Some("support@yourapp.com".into());
let licensing = Licensing::new(config, "/path/to/app-data");

if !licensing.status().licensed {
    licensing.activate(&key).await?;        // Result<LicenseStatus, LicenseError>
}
let check = licensing.launch_check().await; // never errors; Offline when unclear
licensing.release().await?;
```

It's async, using `reqwest` with rustls, so call it from any Tokio context.

## Machine fingerprint

`fingerprint::machine_fingerprint()` is the SHA-256 of `"polar-license-v1:" + OS machine ID`.
The format is frozen by a test, because changing it would make every
existing activation look like a different computer.

## Logging

All events go to the `log` crate (target `polar_license`). With
`log_to_stderr(true)` they're also printed as `[polar-license] …`, which is
handy in `tauri dev`. Polar's raw replies appear here and never in customer
messages.
