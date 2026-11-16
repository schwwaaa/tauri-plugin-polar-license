// tauri-plugin-polar-license — reclaim Worker (Cloudflare Workers, free tier)
//
// ONE job: when an app is told "activation limit reached", release the
// activations on that key that belong to THE SAME COMPUTER that's asking,
// so it can activate again. Activations of other computers are never touched.
//
// Why a server: listing a key's activations requires your Polar
// Organization Access Token. That token must never be inside an app, so it
// lives here as an encrypted Cloudflare secret.
//
// When the app calls it:
//   • its license file was lost (app data deleted, app or OS reinstalled)
//   • a key was disabled and re-enabled after the app marked it deactivated
// In both cases Polar still counts this computer's old activation.
//
// Request:   POST /api/reclaim   { "key": "...", "machine": "<64 hex chars>" }
// Responses (always HTTP 200, JSON):
//   { "state": "reclaimed", "released": n }  → this computer's activation(s) freed
//   { "state": "none" }                      → nothing of this computer's: really another computer
//   { "state": "inactive" }                  → key revoked / disabled / expired / unknown
//   { "state": "error", "message": "..." }   → misconfiguration (the app treats it as "none")
//
// How "this computer" is recognized: at activation the plugin stores a
// SHA-256 hash of the OS machine ID in the activation's meta.machine. The app
// sends the same hash. Matching is exact.
//
// Settings (deploy.sh uploads them):
//   POLAR_API_BASE         https://api.polar.sh  or  https://sandbox-api.polar.sh
//   POLAR_ORGANIZATION_ID  your organization ID
//   POLAR_BENEFIT_IDS      License Keys benefit ID(s), comma-separated (one per app)
//   POLAR_ACCESS_TOKEN     secret; scope license_keys:read is enough

const MACHINE_HASH = /^[0-9a-f]{64}$/;

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", "cache-control": "no-store" },
  });
}

function base(env) {
  return String(env.POLAR_API_BASE || "https://api.polar.sh").replace(/\/$/, "");
}

function benefitIds(env) {
  return String(env.POLAR_BENEFIT_IDS || "").split(",").map((s) => s.trim()).filter(Boolean);
}

async function call(env, method, path, body, withToken) {
  const headers = { accept: "application/json" };
  if (body) headers["content-type"] = "application/json";
  if (withToken) headers.authorization = `Bearer ${env.POLAR_ACCESS_TOKEN}`;
  const res = await fetch(base(env) + path, { method, headers, body: body ? JSON.stringify(body) : undefined });
  let data = null;
  try { data = await res.json(); } catch { /* 204 or not JSON */ }
  return { status: res.status, body: data };
}

export async function reclaim(env, rawKey, rawMachine) {
  const key = String(rawKey || "").trim();
  const machine = String(rawMachine || "").toLowerCase();
  if (!key || key.length > 200 || !MACHINE_HASH.test(machine)) return { state: "error", message: "bad request" };
  if (!env.POLAR_ACCESS_TOKEN || !env.POLAR_ORGANIZATION_ID || !benefitIds(env).length) {
    return { state: "error", message: "worker not configured" };
  }

  // 1. Is the key itself valid, and one of ours? (public endpoint)
  const validated = await call(env, "POST", "/v1/customer-portal/license-keys/validate", {
    key, organization_id: env.POLAR_ORGANIZATION_ID,
  });
  if (validated.status === 404) return { state: "inactive" };
  if (validated.status !== 200) return { state: "error", message: `Polar answered ${validated.status}` };
  if (!benefitIds(env).includes(validated.body.benefit_id)) return { state: "none" };

  // 2. The key's activations (needs the token).
  const lk = await call(env, "GET", `/v1/license-keys/${validated.body.id}`, null, true);
  if (lk.status === 401 || lk.status === 403) return { state: "error", message: "access token needs the license_keys:read scope" };
  if (lk.status !== 200) return { state: "error", message: `Polar answered ${lk.status}` };

  // 3. Release ONLY this computer's activations (public endpoint).
  const mine = (lk.body.activations || []).filter((a) => a.meta && String(a.meta.machine || "").toLowerCase() === machine);
  let released = 0;
  for (const a of mine) {
    const r = await call(env, "POST", "/v1/customer-portal/license-keys/deactivate", {
      key, organization_id: env.POLAR_ORGANIZATION_ID, activation_id: a.id,
    });
    if (r.status === 204 || r.status === 404) released++;
  }
  console.log(`reclaim ****${key.slice(-6)}: matched ${mine.length}, released ${released}`);
  return released ? { state: "reclaimed", released } : { state: "none" };
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/api/reclaim" && request.method === "POST") {
      let body = {};
      try { body = await request.json(); } catch { /* empty */ }
      try {
        return json(await reclaim(env, body.key, body.machine));
      } catch (err) {
        console.error(err);
        return json({ state: "error", message: "internal error" });
      }
    }
    if (url.pathname === "/" && request.method === "GET") {
      return json({ service: "tauri-plugin-polar-license reclaim worker", ok: true });
    }
    return json({ error: "not found" }, 404);
  },
};
