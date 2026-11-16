<div align="center">

# tauri-plugin-polar-license

**Sell your Tauri desktop app with [Polar.sh](https://polar.sh) license keys —
without ever locking a paying customer out of their own software.**

[![CI](https://github.com/schwwaaa/tauri-plugin-polar-license/actions/workflows/ci.yml/badge.svg)](https://github.com/schwwaaa/tauri-plugin-polar-license/actions/workflows/ci.yml)
![Tauri v2](https://img.shields.io/badge/Tauri-v2-24C8DB)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-informational)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

</div>

---

Polar handles payments, taxes and license keys. This plugin handles
everything that happens **inside your app**: the activation screen,
per-computer activations, offline-safe launches, revocation, moving to a new
computer, and recovering a computer that lost its license file.

It was built for live-performance video software, where the one unacceptable
failure is a license check stopping the app mid-show. So the rule is simple:
**once activated, the app always starts, online or not**. Revocations are
picked up quietly in the background and take effect at the next launch.

## Features

- 🔑 **One-time activation per computer**, enforced by Polar's activation limits
- 🛫 **Never blocks a launch.** Licensed launches read a local file and start instantly, even offline
- 🔍 **Background launch check** that catches revoked, disabled and refunded keys, and never interrupts a running session
- 💻 **"Release this machine"**, so customers can move their key to a new computer themselves
- ♻️ **Automatic self-recovery** (optional): a computer that lost its license file gets its *own* activation back, via a tiny free Cloudflare Worker
- 🚦 **Startup gating.** Your main window, renderer or engine starts only once the license is valid (`on_licensed`)
- 🧱 **Drop-in UI**: an activation gate and a license panel, in plain JS and CSS, themeable with CSS variables, safe under a strict CSP
- 🧩 **Typed front-end API** (`tauri-plugin-polar-license-api`) with machine-readable error kinds and customer-ready messages
- 🛡️ **Cross-product protection.** A key bought for one of your apps can't unlock another
- 🧪 **Tested against a fake Polar** that reproduces real production behavior (see [Testing](docs/testing.md))
- 🍎 🪟 🐧 macOS, Windows and Linux

## How it works

```
                 ┌──────────────────────── POLAR ────────────────────────┐
                 │  Product → License Keys benefit → license key         │
                 │  status · activation limit · activations              │
                 └───────▲──────────────────────────────▲────────────────┘
      public endpoints   │                              │  your access token
      (no secret needed) │                              │  (never in the app)
                 ┌───────┴─────────┐           ┌────────┴─────────┐
                 │    YOUR APP     │──────────►│  RECLAIM WORKER  │
                 │ activate        │  only on  │  (optional, free)│
                 │ launch check    │  "limit   │  releases THIS   │
                 │ release         │  reached" │  computer's own  │
                 │ license.json    │           │  old activation  │
                 └─────────────────┘           └──────────────────┘
```

| Moment | What happens | Network? |
|---|---|---|
| First launch | The gate asks for the key. Polar records an **activation** for this computer; the plugin saves it, bound to this computer's fingerprint. | Yes, once |
| Every launch | The plugin reads the saved license. The app starts immediately. | **No** |
| In the background | The **launch check** asks Polar whether the activation is still valid. If it's revoked, the gate appears at the *next* launch. If the result is unclear (offline), nothing changes. | If available |
| Changing computers | **Release this machine** frees the slot. | Yes |
| License file lost | Polar still counts the old activation. The Worker recognizes this computer's activation and frees it, and activation succeeds. | Yes |

Full walkthroughs: [How it works](docs/how-it-works.md).

## Quick start

### 1. Polar (once per app)

Create a product with a **License Keys** benefit and **turn the activation
limit on** (Polar refuses activation on keys without one). Note your
**organization ID** and the **benefit ID**. Do it in the sandbox for testing
and again in production; the two share nothing.
→ [Step-by-step Polar setup](docs/polar-setup.md)

### 2. Rust

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-polar-license = { git = "https://github.com/schwwaaa/tauri-plugin-polar-license" }
```

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_polar_license::Builder::new()
                .production("POLAR-ORG-ID", "LICENSE-KEYS-BENEFIT-ID")
                .sandbox("SANDBOX-ORG-ID", "SANDBOX-BENEFIT-ID")
                .use_sandbox(cfg!(debug_assertions))
                .support_contact("support@yourapp.com")
                // Optional: automatic recovery (see docs/worker.md)
                .reclaim_url("https://polar-license.you.workers.dev")
                // Start your real app only once licensed:
                .on_licensed(|app| open_main_window(app))
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running app");
}
```

`on_licensed` runs **once**, on the main thread: at startup if this computer
is already licensed, or right after a successful activation. Open your main
window, start your renderer or audio engine there.

### 3. Permissions

```jsonc
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": ["core:default", "polar-license:default"]
}
```

### 4. Front end

**Option A: the drop-in UI** (no framework, no build step):

```html
<link rel="stylesheet" href="polar-license.css">
<script src="polar-license.js"></script>
<script>
  PolarLicenseUI.mount({
    appName: 'My App',
    onLicensed: () => startUi(),      // licensed at launch, or just activated
    onReleased: () => quitApp(),      // after "Release this machine"
  });
  document.querySelector('#license-button')
    .addEventListener('click', () => PolarLicenseUI.openPanel());
</script>
```

Copy `ui/polar-license.js` and `ui/polar-license.css` into your front end. Set
`"withGlobalTauri": true`, or pass `{ invoke }`.

**Option B: your own UI** with the typed API:

```ts
import { getStatus, activate, release, getLaunchCheck, type LicenseError } from 'tauri-plugin-polar-license-api';

const status = await getStatus();
if (!status.licensed) {
  try {
    await activate(key);
  } catch (e) {
    const err = e as LicenseError;   // { kind: 'inUseElsewhere' | 'deactivated' | …, message }
    show(err.message);               // customer-ready sentence
  }
}
```

→ [Front-end API and UI reference](docs/frontend.md)

## What customers see

| Situation | Message (`kind`) |
|---|---|
| Typo / unknown key | That license key wasn't recognized. Check that you pasted the whole key. (`notRecognized`) |
| Key from another of your products | This license key is for a different product… (`wrongProduct`) |
| All activation slots taken | This license key is already activated on another computer… click "Release this machine"… (`inUseElsewhere`) |
| Key revoked or disabled | This license key has been deactivated. (`deactivated`) |
| No connection | Couldn't connect to the license server… (`network`) |

Each message ends with *"Need help? Contact &lt;support_contact&gt;."* when one
is set. Polar's raw replies are never shown to customers; they go to the
developer log (`[polar-license] …`).

## Before you ship: the five things that bite

1. **Revoke is permanent; disable is reversible.** Polar's docs don't say this; production testing showed it. Use *disable* for anything temporary. → [Caveat C4](docs/caveats.md#c4-revoke-is-permanent-disable-is-reversible)
2. **Activations never expire on their own.** A customer who wipes their laptop still holds a slot. Release it for them, or run the Worker. → [C6](docs/caveats.md#c6-activations-never-expire-on-their-own), [C7](docs/caveats.md#c7-a-lost-license-file-blocks-the-same-computer)
3. **Sandbox and production share nothing.** A production key in a sandbox build is "not recognized". → [C1](docs/caveats.md#c1-sandbox-and-production-are-separate-worlds)
4. **Never change the bundle identifier, organization ID or benefit ID after shipping.** Every customer's saved license would stop matching. → [C15](docs/caveats.md#c15-never-change-these-after-shipping)
5. **Dashboard refunds revoke the key; API refunds don't, unless you ask.** → [C10](docs/caveats.md#c10-refund-defaults-differ-between-dashboard-and-api)

All 19 caveats, each with its impact on you: **[docs/caveats.md](docs/caveats.md)**

## Security model

- The app uses **only Polar's public license endpoints**. No secret is ever compiled into your app.
- Keys are accepted **only for your benefit ID**, so they can't cross between your products.
- The license file is **bound to the computer** (SHA-256 of the OS machine ID); a copied file doesn't work elsewhere.
- Only a **genuine Polar "not found"** removes a license. A captive-portal page, proxy error or outage never does.
- Debug builds can be pointed at a local fake Polar (`POLAR_LICENSE_API_BASE_OVERRIDE`); **release builds ignore it.**
- **Honest limits:** like every client-side license check, the app binary can be patched, and the license file isn't signed. The launch check catches a forged file whenever that computer is online. The activation limit plus [activation tracking](docs/tracking-activations.md) are your real controls.

Report vulnerabilities privately: [SECURITY.md](SECURITY.md).

## Documentation

| Doc | What's in it |
|---|---|
| [Polar setup](docs/polar-setup.md) | Every step in Polar, sandbox and production: products, benefits, IDs, tokens, test purchases, refunds |
| [How it works](docs/how-it-works.md) | Lifecycle, data stored, and five customer walkthroughs |
| [Caveats](docs/caveats.md) | 19 caveats, each with its impact on you |
| [Rust API](docs/rust-api.md) | `Builder`, `PolarLicenseExt`, standalone `Licensing` |
| [Front end](docs/frontend.md) | TypeScript API, drop-in UI, theming, capabilities |
| [Reclaim Worker](docs/worker.md) | Deploying the optional Cloudflare Worker, step by step |
| [Tracking activations](docs/tracking-activations.md) | Seeing who activated where, and what sharing looks like |
| [Support playbook](docs/support-playbook.md) | Common customer tickets and what to do |
| [Testing](docs/testing.md) | The test suite, the fake Polar, testing with real Polar |
| [Terminology](docs/terminology.md) | Every Polar and plugin term |

## Platform support

| | macOS | Windows | Linux |
|---|---|---|---|
| Activation, launch check, release | ✅ | ✅ | ✅ |
| Machine fingerprint source | IOPlatformUUID (hardware) | MachineGuid (registry) | /etc/machine-id |
| Fingerprint survives OS reinstall | ✅ | ❌ (new ID) | ❌ (new ID) |
| Reclaim after app-data loss / reinstall | ✅ | ✅ | ✅ |
| Reclaim after OS reinstall | ✅ | manual release | manual release |

Requires Tauri **2.x** and Rust **1.77.2+**.

## FAQ

**Does the app need internet to start?**
Only for the very first activation. After that it starts offline, forever.

**What happens when I refund someone?**
A dashboard refund revokes the key by default. At their next launch with a connection, the app notes it; the launch after that shows the gate. A running session is never interrupted.

**Can I let customers use two computers?**
Yes. Set the activation limit on the benefit, or per key via Polar's API. Everything in the plugin works with any limit.

**Do I need the Worker?**
No. Without it, a customer whose license file was lost (reinstall, wiped app data) is told "already activated on another computer", and you release the old activation for them. The Worker turns that into automatic recovery. It's free at any realistic volume.

**Can I issue a replacement key manually?**
Polar has no "create key" button or API call. Use a 100% discount code, or a private free product with the same License Keys benefit. → [Polar setup §8](docs/polar-setup.md#8-issuing-keys-without-a-normal-purchase)

**Electron?**
Not yet. The core (`Licensing`) has no Tauri dependency, so other front ends are on the roadmap.

## Roadmap

- [ ] Publish to crates.io and npm
- [ ] Optional signed license file (offline forgery resistance) via the Worker
- [ ] Electron / Node bindings
- [ ] Localized customer messages

## Contributing

Issues and PRs are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). Run
`cargo test` and `npm test --prefix worker` before submitting.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

*Not affiliated with Polar Software Inc. "Polar" is their trademark.*
