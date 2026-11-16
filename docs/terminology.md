# Terminology

> **Source tags.** **[Docs]** = stated in Polar's or Cloudflare's documentation.
> **[Tested]** = observed in real sandbox/production testing (Nov 2026).
> **[Design]** = how this plugin behaves, by choice.

### Polar side

| Term | Meaning |
|---|---|
| **Organization** | Your seller account in Polar. Identified by an **organization ID** (a UUID). |
| **Sandbox** | Polar's test environment (`sandbox.polar.sh`, API `sandbox-api.polar.sh`): fake money and test cards. **A completely separate world** from production, with its own organization, products, IDs and keys. [Docs] |
| **Production** | The real Polar (`polar.sh`, API `api.polar.sh`). Real money. |
| **Product** | What a customer buys, e.g. "My App Pro". Has a price (or is free) and one or more benefits. |
| **Benefit** | Something a customer gets by buying a product. Each benefit has a **benefit ID**. |
| **License Keys benefit** | The benefit type that issues a license key. Its settings: activation limit, whether customers can deactivate, expiration, usage limit. [Docs] |
| **Benefit grant** | The record that a specific customer received a specific benefit. A License Keys grant points to the license key it created. |
| **License key** | The record Polar creates for a buyer. It has an ID, a **key string** (what the customer types), a **display key** (masked, e.g. `****-E304DA`), a **status**, an activation limit, and a list of activations. |
| **Status** | `granted`, `revoked` or `disabled`. [Docs] See [C4](caveats.md#c4-revoke-is-permanent-disable-is-reversible) and [C5](caveats.md#c5-rotating-changes-the-string-only) for how they actually behave. |
| **Activation** | One computer's claim on a key. Has an **activation ID**, a **label** and **meta** (custom data). Counts toward the activation limit until it's released. |
| **Activation limit** | The maximum number of simultaneous activations per key, set on the benefit (and changeable per key via the API). [Docs] |
| **Release** (Polar: *deactivate*) | Deleting one activation, which frees its slot. |
| **Revoke** | Setting a key's status to revoked. In practice, permanent. [Tested] |
| **Disable** | Setting a key's status to disabled. Reversible. [Tested] |
| **Rotate** | Replacing a key's string with a new one. The license record, its status and its activations stay. [Docs + Tested] |
| **Validate** | Asking Polar "is this key (and optionally this activation) valid right now?" |
| **Customer portal** | Polar's page for buyers at `polar.sh/<org-slug>/portal`. Signing in uses a one-time email code. |
| **Checkout link** | A shareable link that opens Polar's checkout for a product. Can have a **Success URL** (where buyers go after paying). [Docs] |
| **Discount code** | A percentage or fixed reduction. Can be restricted to products and capped in redemptions. [Docs] |
| **Organization Access Token** (`polar_oat_…`) | A secret API token for your organization, with chosen **scopes** (e.g. `license_keys:read`). **Must never be inside an app.** |

### Plugin side

| Term | Meaning |
|---|---|
| **License file** (`license.json`) | What the app saves after activating: key, activation ID, environment, benefit, machine fingerprint, revoked flag. Stored in the OS app-data folder. [Design] |
| **Machine fingerprint** | A SHA-256 hash of the OS machine ID (macOS hardware UUID, Windows MachineGuid, Linux `/etc/machine-id`). Stored in the license file and in the activation's `meta.machine` in Polar. The raw ID is never sent. [Design] |
| **Label** | The computer's name, sent with each activation so you can recognize it in Polar. [Design] |
| **Gate** | The activation popup. The app's main window/engine doesn't start until the license is valid. [Design] |
| **Launch check** | A background validation at each launch, after the app has already started. Never blocks. [Design] |
| **Reclaim** | Releasing *this same computer's* old activation so it can activate again. Needs the [Worker](worker.md). [Design] |
| **Worker** | An optional small server (Cloudflare Worker, free tier) that holds your access token and performs reclaim. [Design] |
| **Environment** | Which Polar a build talks to: sandbox or production. Chosen at build time. [Design] |

---

