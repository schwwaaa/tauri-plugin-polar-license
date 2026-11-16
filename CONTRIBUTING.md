# Contributing

Thanks for helping. Bug reports, docs fixes and PRs are all welcome.

## Before opening a PR

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
npm install && npm run build      # TypeScript bindings
npm test --prefix worker          # reclaim Worker
```

CI runs the same checks on macOS, Windows and Linux.

## Ground rules

- **Never block a licensed launch.** Anything network-related at launch must run in the background and only act on a *definitive* Polar answer.
- **Customer messages** live in `src/messages.rs`. Keep them plain, accurate, and limited to things that exist in the app. Polar's raw text goes to the log, never to customers.
- **Compatibility:** the license file format (`FILE_VERSION`) and the machine fingerprint format are frozen. Changing either would orphan existing customers' activations; it needs a migration and a major version.
- **Behavior claims about Polar** in docs must be tagged **[Docs]** (with a link) or **[Tested]** (observed against real Polar).
- New behavior needs a scenario in `tests/scenarios.rs`. If it depends on how Polar behaves, model that in the fake (`tests/common/mod.rs`) and say where the behavior comes from.

## Reporting bugs

Please include: OS, Tauri version, plugin version, the `[polar-license]`
log lines (remove full keys), and what the customer saw.
