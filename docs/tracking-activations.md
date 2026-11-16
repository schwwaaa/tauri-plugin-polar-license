# Tracking activations

The activation limit is your main control against key sharing. This page covers what Polar records for each key, what sharing looks like in that data, and what you can do about it.

Polar keeps, for every key:

| Data | Where it comes from |
|---|---|
| status (`granted` / `revoked` / `disabled`) | Polar |
| activation limit | benefit, or per key |
| each activation's **label** | computer name, sent by the app |
| each activation's **meta.machine** | fingerprint, sent by the app |
| each activation's **created_at** | Polar |
| buyer email | the order |

**What sharing looks like in that data:**
- **The label or machine fingerprint keeps changing** on one key: releases and re-activations alternating between different computers ([walkthrough 5.3](how-it-works.md#53-someone-shares-their-key)).
- **Many activations created over a short time** on one key.
- **Keys at their limit**, combined with "another computer" support requests from someone who isn't the buyer.

**What you can do about it:**
- **Disable** the key while you look into it (reversible).
- **Revoke** it if it's confirmed (permanent).
- **Lower that key's activation limit** (`PATCH /v1/license-keys/{id}`).

**How to read it** (token with `license_keys:read`): `GET /v1/license-keys/{id}` returns the activations with labels and `meta` (needs `license_keys:read`). `GET /v1/license-keys?organization_id=…&benefit_id=…` lists all keys.

---

