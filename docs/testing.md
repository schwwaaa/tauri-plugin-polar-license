# Testing

## The automated suite

```bash
cargo test                # Rust: unit tests + customer scenarios
npm test --prefix worker  # reclaim Worker
npm run build             # TypeScript bindings compile
```

No network or Polar account is needed. The Rust tests start a **fake Polar**
in-process (`tests/common/mod.rs`) that reproduces what real Polar did in
production testing:

| Behavior | Source |
|---|---|
| activations count toward the limit until released | Polar docs |
| `validate` answers 404 `ResourceNotFound` for revoked, disabled or expired keys and for released activations | Polar docs |
| **revoke is permanent**: "enable" afterwards changes nothing; its activations stay counted but never validate again | production testing |
| **disable is reversible**: re-enable and the key and its activations work again | production testing |
| **rotate** changes the key string; status and activations stay | Polar docs + production testing |
| unknown paths return an HTML 404, like a proxy or captive portal | to test that only genuine Polar 404s remove a license |

It also implements the reclaim Worker's logic, so reclaim is tested end to end.

### Scenarios covered (`tests/scenarios.rs`)

| Test | Story |
|---|---|
| `buys_and_activates` | purchase → activate → licensed from the file alone |
| `launch_check_confirms_offline_and_ignores_stray_404s` | confirmed online; offline and non-Polar 404s change nothing |
| `typo_and_empty_key` | clear messages, support line appended |
| `key_for_another_product_is_refused_and_slot_returned` | cross-product protection; slot handed back |
| `not_configured_is_reported` | placeholder IDs detected |
| `second_computer_is_refused_and_first_stays_licensed` | limit enforced; reclaim never takes another computer's slot |
| `copied_license_file_does_not_work_on_another_computer` | fingerprint binding |
| `moves_to_a_new_computer_with_release` | release → activate elsewhere |
| `lost_license_file_is_recovered_by_reclaim` | wipe → same computer gets back in, still 1 activation |
| `lost_license_file_without_worker_needs_a_manual_release` | the case the Worker exists for |
| `disabled_then_re_enabled_key_works_again` | gate after disable; reuses its own activation after re-enable |
| `revoked_key_stays_dead_even_after_enable_or_rotate` | revoke is final |
| `rotated_key_on_a_lost_file_computer_is_reclaimed` | new key string, same computer |
| `activation_limit_above_one` | limit of 3 enforced |

## Testing your app against a local fake Polar

Debug builds honor `POLAR_LICENSE_API_BASE_OVERRIDE`:

```bash
POLAR_LICENSE_API_BASE_OVERRIDE=http://127.0.0.1:8787 npm run tauri dev
```

Release builds ignore it, so a shipped app can't be redirected.

## Testing with real Polar

**Sandbox** (free, test card):
1. Build with `.use_sandbox(true)`. Dev builds show "Test build — using the Polar sandbox." on the gate.
2. Buy with `4242 4242 4242 4242`. Get the key from your seller dashboard, or by email only if you bought with the organization **owner's** address ([C2](caveats.md#c2-sandbox-emails-only-reach-organization-members)).

**Production** (no charge): a free product with the same benefit, or a 100% discount code ([Polar setup §8](polar-setup.md#8-issuing-keys-without-a-normal-purchase)).

**One key covers every scenario:**
1. Activate.
2. Release in the panel, then activate again.
3. Disable it in Polar: at the next launch the panel says deactivated, and the launch after that shows the gate.
4. Re-enable it: the key activates again.
5. Revoke only at the very end, because a revoke is permanent.

## CI

`.github/workflows/ci.yml` runs formatting, Clippy (warnings are errors),
the tests and the docs build on **macOS, Windows and Linux**, plus the Worker
tests and the TypeScript build.
