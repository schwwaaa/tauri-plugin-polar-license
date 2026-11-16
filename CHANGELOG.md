# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-11-15

### Added
- Tauri v2 plugin `polar-license`: activate, release, status and launch-check commands, with permissions (`polar-license:default`).
- `Builder` with production and sandbox environments, `on_licensed` startup gating (main thread, once per process), reclaim URL, support contact, stderr logging and license file name.
- `PolarLicenseExt` for `app.polar_license()`.
- Standalone `Licensing` core usable without Tauri.
- License file bound to a SHA-256 machine fingerprint; atomic writes; environment and benefit checks.
- Non-blocking launch check: only genuine Polar `ResourceNotFound` replies mark a license deactivated, effective at the next launch.
- Reclaim: automatic recovery of a computer's own activation via an optional Cloudflare Worker (`worker/`), with a curl-only `deploy.sh`.
- Typed errors (`ErrorKind`) with customer-ready messages and an optional support line.
- TypeScript bindings (`tauri-plugin-polar-license-api`).
- Drop-in UI: activation gate and license panel, themeable via CSS variables, CSP-safe.
- Fake-Polar test suite reproducing production behavior (revoke permanent, disable reversible, rotation keeps activations).
- Documentation: Polar setup, how it works, caveats, Rust API, front end, Worker, tracking activations, support playbook, testing, terminology.
