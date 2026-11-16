# Polar setup: the complete process

Everything you do **in Polar** to sell a desktop app with license keys, from
an empty account to handling refunds. You'll do most of it twice: once in the
**sandbox** for testing and once in **production** for selling.

> **Source tags.** **[Docs]** = stated in Polar's documentation.
> **[Tested]** = observed in real sandbox/production testing (Nov 2026), not
> stated in the docs. Polar's dashboard changes over time, so where its exact
> menu layout isn't documented, this guide names the thing to look for
> instead of guessing click paths.

---

## Contents

1. [Sandbox vs production](#1-sandbox-vs-production)
2. [Create the product](#2-create-the-product)
3. [Add the License Keys benefit](#3-add-the-license-keys-benefit)
4. [Collect the IDs the plugin needs](#4-collect-the-ids-the-plugin-needs)
5. [Create an access token (only for the Worker or your own tooling)](#5-create-an-access-token-only-for-the-worker-or-your-own-tooling)
6. [Make a test purchase](#6-make-a-test-purchase)
7. [Managing keys: disable, revoke, rotate, limits](#7-managing-keys-disable-revoke-rotate-limits)
8. [Issuing keys without a normal purchase](#8-issuing-keys-without-a-normal-purchase)
9. [Refunds](#9-refunds)
10. [Releasing a customer's activation](#10-releasing-a-customers-activation)
11. [Going live checklist](#11-going-live-checklist)

---

## 1. Sandbox vs production

| | Sandbox | Production |
|---|---|---|
| Dashboard | `sandbox.polar.sh` | `polar.sh` |
| API | `https://sandbox-api.polar.sh` | `https://api.polar.sh` |
| Money | fake; test card `4242 4242 4242 4242`, any future expiry, any CVC [Docs] | real cards only; the test card is refused |
| Emails to customers | only delivered to **members of your organization** [Docs]. In testing, only the **owner** email received the key email [Tested]. | delivered to buyers |

**They share nothing:** separate organizations, products, benefit IDs and
keys [Docs]. A sandbox key is "not recognized" by a production build, and
vice versa. The plugin keeps a separate set of IDs for each:

```rust
.production("PROD-ORG-ID", "PROD-BENEFIT-ID")
.sandbox("SANDBOX-ORG-ID", "SANDBOX-BENEFIT-ID")
.use_sandbox(cfg!(debug_assertions))   // dev builds → sandbox
```

---

## 2. Create the product

1. In the dashboard of the environment you're setting up, create a **new product**.
2. **Name:** what customers see at checkout and on receipts.
3. **Pricing:** one-time paid for a perpetual license, or **Free**. Free products never ask for a card [Docs]. That's useful for testing, see §8.
4. Save.

Each app you sell gets its own product, and its own License Keys benefit
(next step).

---

## 3. Add the License Keys benefit

A *benefit* is what a buyer gets. The **License Keys** benefit makes Polar
issue a key per purchase [Docs].

1. Add a benefit to the product, of type **License Keys**.
2. Settings [Docs]:

| Setting | What to choose | Why |
|---|---|---|
| **Activation limit** | **On**, set to how many computers a buyer may use at once (1, 2, 3…) | **Required.** Polar refuses activation on keys that don't support activations. Each computer that activates uses one slot until it's released. |
| **Allow customers to deactivate** | On | Lets customers release their own activations from Polar's customer portal. In testing the portal didn't show keys at all [Tested], so don't rely on it; the app's "Release this machine" always works. |
| **Expiration** | Off, for perpetual licenses | An expired key fails validation, and the app treats it as deactivated. |
| **Usage limit** | Off | Not used by this plugin. |

3. Save, and make sure the benefit is attached to the product.

**One benefit per app.** The plugin only accepts keys from *its* benefit ID,
so a key bought for app A can't unlock app B, even within one organization.

---

## 4. Collect the IDs the plugin needs

| ID | Where to find it | Goes into |
|---|---|---|
| **Organization ID** | Organization **Settings** (general settings page) | `.production(org, …)` / `.sandbox(org, …)`, and the Worker's `POLAR_ORGANIZATION_ID` |
| **Benefit ID** | Open the License Keys benefit; its ID is on its page or in the address bar | `.production(…, benefit)` / `.sandbox(…, benefit)`, and the Worker's `POLAR_BENEFIT_IDS` |

Neither is secret. Collect both in **each** environment.

---

## 5. Create an access token (only for the Worker or your own tooling)

The app **never** uses a token. You need one only to run the reclaim
Worker, or to read and manage keys with your own scripts.

1. Organization **Settings** → **Organization Access Tokens** → create.
2. **Scopes:**
   - reclaim Worker: **`license_keys:read`** only;
   - your own key-management scripts: add `license_keys:write` (to change status or limits).
3. **Expiration:** the longest available. When it expires, the Worker stops reclaiming, and the app falls back to the "another computer" message.
4. Copy it. It starts with `polar_oat_` and is shown once.

Treat it like a password: never commit it, never put it in an app.

---

## 6. Make a test purchase

**Sandbox**
1. Open the product's **checkout link** in a private browser window.
2. Pay with `4242 4242 4242 4242`, any future expiry, any CVC.
3. Get the key:
   - **by email:** only reliable for the organization **owner's** address in the sandbox [Tested];
   - **from your seller dashboard:** open the order or customer, or the license keys list;
   - **through the API:** `GET /v1/license-keys?organization_id=…` with a token (the response includes the full `key`).

**Production, without paying.** Use a free product or a 100% discount code (§8). This is the only way to see exactly what a real customer receives.

---

## 7. Managing keys: disable, revoke, rotate, limits

A key has a **status**: `granted`, `revoked` or `disabled` [Docs]. Polar's
docs don't say what each means or which changes are allowed. Here's what
production testing showed:

| Action | Effect | Reversible? | Activations |
|---|---|---|---|
| **Disable** | Activation and validation are refused. The app shows "deactivated" from the next launch. | **Yes.** Re-enable and the key works again, on the same computer, without using a new slot [Tested]. | kept |
| **Revoke** | Same as disable, but **permanent**. Setting it back to active doesn't revive it [Tested]. | **No** | kept, still counted |
| **Rotate** | The key gets a **new string**; the old string stops working. Customers can also rotate from the portal [Docs]. | n/a | **unchanged.** Rotating doesn't free slots or revive a revoked key [Tested]. |
| **Change activation limit** | `PATCH /v1/license-keys/{id}` with `limit_activations` [Docs] | yes | unchanged |

**Rule of thumb:** *disable* for anything temporary or uncertain (payment
dispute, investigation); *revoke* only for final decisions (refund,
confirmed abuse).

API (token with `license_keys:write`):

```bash
# Disable a key
curl -X PATCH https://api.polar.sh/v1/license-keys/KEY_ID \
  -H "Authorization: Bearer $POLAR_TOKEN" -H "Content-Type: application/json" \
  -d '{"status":"disabled"}'

# Re-enable it
curl -X PATCH https://api.polar.sh/v1/license-keys/KEY_ID \
  -H "Authorization: Bearer $POLAR_TOKEN" -H "Content-Type: application/json" \
  -d '{"status":"granted"}'

# Allow this one customer 3 computers
curl -X PATCH https://api.polar.sh/v1/license-keys/KEY_ID \
  -H "Authorization: Bearer $POLAR_TOKEN" -H "Content-Type: application/json" \
  -d '{"limit_activations":3}'
```

---

## 8. Issuing keys without a normal purchase

Polar has **no way to create a license key directly**: no dashboard button,
no API endpoint. Keys are *"automatically generated and granted to customers
who purchase"* [Docs]. To give someone a key, they need an order:

| Method | How | Notes |
|---|---|---|
| **Free product, same benefit** | Create a second product priced Free, with the **same** License Keys benefit as the paid one | Keys work identically, because the app checks the benefit, not the product. Keep its checkout link private; archive it when not needed. |
| **100% discount code** | Create a percentage discount of 100%, restricted to your product, with a low max-redemptions cap [Docs] | Tests the real product. The docs guarantee no card form only for free products and 100%-forever subscriptions, so check the checkout total shows $0 before continuing. |

Typical uses: replacing a revoked key, press and review copies, giveaways,
your own production testing.

---

## 9. Refunds

| How you refund | What happens to the key [Docs] |
|---|---|
| Dashboard | "Revoke benefits" is **selected by default**, so the key is revoked |
| API (`POST /v1/refunds`) | `revoke_benefits` **defaults to false**, so the key keeps working unless you send `true`. Only allowed for one-time orders. |
| Polar-initiated (to prevent a chargeback) | benefits are revoked |
| Partial refund | not documented |

In the app, a revoked key is picked up by the next launch check while
online, and the gate appears at the launch after that. A machine that never
goes online again keeps working (by design: see
[caveat C11](caveats.md#c11-revocations-reach-the-app-only-at-launch-while-online)).

---

## 10. Releasing a customer's activation

The case: a customer's computer died, was wiped or was sold, and the app on
their new computer says "already activated on another computer".

1. **Ideal:** they click **Release this machine** in the app's license panel on the old computer, if they still have it.
2. **With the reclaim Worker:** if it's the *same* computer after a wipe or reinstall, it fixes itself.
3. **Otherwise, you release it:**
   - **API, always works.** Find the key:
     ```bash
     curl "https://api.polar.sh/v1/license-keys?organization_id=$ORG" -H "Authorization: Bearer $POLAR_TOKEN"
     ```
     Then list its activations, each with a `label` (the computer's name) and `meta.machine`:
     ```bash
     curl "https://api.polar.sh/v1/license-keys/KEY_ID" -H "Authorization: Bearer $POLAR_TOKEN"
     ```
     Then release one:
     ```bash
     curl -X POST https://api.polar.sh/v1/customer-portal/license-keys/deactivate \
       -H "Content-Type: application/json" \
       -d '{"key":"FULL-KEY","organization_id":"ORG","activation_id":"ACTIVATION_ID"}'
     ```
     A successful release returns `204`.
   - **Dashboard:** Polar's docs don't describe where activations appear in the dashboard. Find it once in your account before you need it for support.

---

## 11. Going live checklist

- [ ] Production product created, with a **License Keys** benefit, activation limit **on**
- [ ] Production organization ID and benefit ID in `.production(…)`
- [ ] Release build uses production (`.use_sandbox(false)` in release)
- [ ] `.support_contact(…)` set to an address you read
- [ ] One real end-to-end purchase in production (free product or 100% code), activated in a **release** build
- [ ] You know how to release an activation (§10)
- [ ] You know the difference between disable and revoke (§7)
- [ ] (Optional) Worker deployed against **production** with a production token; `.reclaim_url(…)` set
- [ ] Bundle identifier, organization ID and benefit ID are final. They must never change after shipping.

---

**Sources:** [License Keys](https://polar.sh/docs/features/benefits/license-keys) ·
[License keys API](https://www.mintlify.com/polarsource/polar/api-reference/license-keys) ·
[Update license key](https://polar.sh/docs/api-reference/license_keys/update-license-key) ·
[Activate](https://polar.sh/docs/api-reference/customer-portal/license-keys/activate) ·
[Validate](https://polar.sh/docs/api-reference/customer-portal/license-keys/validate) ·
[Deactivate](https://polar.sh/docs/api-reference/license_keys/deactivate-license-key) ·
[Refunds](https://polar.sh/docs/features/refunds) ·
[Create refund](https://polar.sh/docs/api-reference/refunds/create-refund.md) ·
[Discounts](https://polar.sh/docs/features/discounts.md) ·
[Products](https://polar.sh/docs/features/products.md) ·
[Sandbox](https://polar.sh/docs/integrate/sandbox) ·
[Customer portal](https://polar.sh/docs/features/customer-portal/introduction.md)
