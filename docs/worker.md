# Reclaim Worker (optional)

## What it's for

An activation counts against the key's limit until it's released. If a
computer **loses its license file** (app data deleted, app reinstalled, or a
macOS reinstall), Polar still counts that computer's old activation, so the
same computer is refused with "limit reached", blocked by itself. The same
happens when a key that the app had marked deactivated is re-enabled.

The Worker fixes that automatically. The app sends it the key plus this
computer's fingerprint. The Worker lists the key's activations, releases
**only the ones stamped with that same fingerprint**, and the app activates
again.

**Why a server:** listing a key's activations needs your Polar access token,
and a token must never be inside an app. The Worker keeps it as an encrypted
Cloudflare secret.

**Without it:** the customer sees "already activated on another computer",
and you release the old activation for them ([Polar setup §10](polar-setup.md#10-releasing-a-customers-activation)).

## What it will and won't do

| Request from the app | Worker answer | App shows |
|---|---|---|
| Same computer, key valid | `reclaimed`: this computer's activation released | activates normally |
| Different computer | `none`: nothing released | "already activated on another computer" |
| Key revoked, disabled or expired | `inactive`: nothing released | "This license key has been deactivated." |
| Worker down or misconfigured | (no answer / `error`) | "already activated on another computer" |

One Worker serves **all your apps** in one Polar organization: list every
app's benefit ID in `POLAR_BENEFIT_IDS`.

## Cost

Cloudflare's Workers Free plan: **100,000 requests per day per account**
(shared by all your Workers) and **100 Workers per account**. The Worker is
only called when an activation hits "limit reached".
([pricing](https://developers.cloudflare.com/workers/platform/pricing/), [limits](https://developers.cloudflare.com/workers/platform/limits/))

## Deploy, step by step

Everything runs from the `worker/` folder with `bash`, `curl` and `python3`.
These are preinstalled on macOS and most Linux; on Windows, use Git Bash or
WSL. No Wrangler or npm install is needed.

### 1. Cloudflare account and workers.dev subdomain (browser)

1. Sign up at **dash.cloudflare.com** (free plan).
2. Open **Workers & Pages**. If asked, choose a **workers.dev subdomain** (e.g. `yourname`). It becomes part of the address: `https://polar-license.yourname.workers.dev`.

### 2. Cloudflare API token (browser)

1. Profile icon → **My Profile** → **API Tokens** → **Create Token**.
2. Under the policy templates, choose **Edit Cloudflare Workers**.
3. **Account Resources:** Include → *your account* (the one named after your login email).
   **Zone Resources:** Include → All zones (required by the template; the Worker uses no zones).
4. **Continue to summary** → **Create Token** → copy it (shown once).

### 3. Polar access token (browser)

In the Polar environment the Worker should serve (production or sandbox):
Settings → **Organization Access Tokens** → create, with scope
**`license_keys:read`**. Copy it (`polar_oat_…`, shown once).

### 4. Settings file (terminal)

```bash
cd worker
cp config.env.example config.env
```

Edit `config.env`:

| Setting | Value |
|---|---|
| `CF_ACCOUNT_ID` | the code in the dashboard address bar: `dash.cloudflare.com/<THIS>/…` |
| `WORKER_NAME` | `polar-license` (or anything: it becomes part of the address) |
| `POLAR_API_BASE` | `https://api.polar.sh` (or `https://sandbox-api.polar.sh`) |
| `POLAR_ORGANIZATION_ID` | your organization ID **in that environment** |
| `POLAR_BENEFIT_IDS` | your License Keys benefit ID(s), comma-separated |

### 5. Publish (terminal)

```bash
chmod +x deploy.sh   # first time only
./deploy.sh
```

The first run asks for the two tokens. **Typing is hidden**: paste each one
and press Return. They're saved in `secrets.env`, readable only by your
user account, and sent only to Cloudflare, which stores the Polar token
encrypted.

Expected output ends with:

```
5/5  Done
     Worker address — put it in your app's plugin config:
         .reclaim_url("https://polar-license.yourname.workers.dev")
     Checking the Worker answers...
     https://polar-license.yourname.workers.dev answers — live.
```

Re-run `./deploy.sh` after any change to `config.env` or `src/worker.js`.
Change the tokens with `./deploy.sh --reset-secrets`.

### 6. Point the app at it

```rust
.reclaim_url("https://polar-license.yourname.workers.dev")
```

Use a Worker that talks to the **same Polar environment** as the build:
production builds use a production Worker. For both, deploy two copies with
different `WORKER_NAME`s.

## Errors from deploy.sh

| Message | Fix |
|---|---|
| `config.env: X still has a PASTE-… placeholder` | Fill in that line |
| `Cloudflare refused the API token` | Re-create it from the **Edit Cloudflare Workers** template; then `./deploy.sh --reset-secrets` |
| `Cloudflare doesn't know that account ID` | Check `CF_ACCOUNT_ID` |
| `no workers.dev subdomain yet` | Open Workers & Pages in the dashboard once and choose one |

## Checking it from the app side

With `log_to_stderr` on (the default in debug builds), every reclaim prints
one line:

```
[polar-license] reclaim: {"state":"reclaimed","released":1}
[polar-license] reclaim: {"state":"none"}
[polar-license] reclaim: {"state":"inactive"}
[polar-license] reclaim: Worker unreachable
[polar-license] reclaim: off (no reclaim_url configured)
```

## Privacy

The Worker receives a license key and a 64-character hash, never the raw
machine ID. It logs only the last six characters of the key, and how many
activations matched and were released.
