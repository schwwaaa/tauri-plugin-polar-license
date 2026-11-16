// Tests for the reclaim Worker against a local stand-in for Polar's API.
// Run:  npm test   (Node 18+)
import http from "node:http";
import worker, { reclaim } from "../src/worker.js";

const TOKEN = "polar_oat_test";
const ORG = "00000000-0000-4000-8000-000000000000";
const BENEFIT = "11111111-1111-4111-8111-111111111111";
const MINE = "a".repeat(64), OTHER = "b".repeat(64);
const KEY = "RECLAIM-KEY", KEY_ID = "lk-1";
let activations = [
  { id: "act-mine", label: "Studio-Mac", meta: { machine: MINE } },
  { id: "act-other", label: "Show-Laptop", meta: { machine: OTHER } },
];
const released = [];

const polar = http.createServer(async (req, res) => {
  const send = (code, body) => { res.writeHead(code, { "content-type": "application/json" }); res.end(body === undefined ? "" : JSON.stringify(body)); };
  let d = ""; for await (const c of req) d += c;
  const b = JSON.parse(d || "{}");
  if (req.method === "POST" && req.url === "/v1/customer-portal/license-keys/validate") {
    if (b.organization_id !== ORG) return send(404, { error: "ResourceNotFound" });
    if (b.key === KEY) return send(200, { id: KEY_ID, status: "granted", benefit_id: BENEFIT });
    if (b.key === "OTHER-APP-KEY") return send(200, { id: "lk-x", status: "granted", benefit_id: "not-this-app" });
    return send(404, { error: "ResourceNotFound" });
  }
  if (req.method === "POST" && req.url === "/v1/customer-portal/license-keys/deactivate") {
    const before = activations.length;
    activations = activations.filter((a) => a.id !== b.activation_id);
    if (activations.length === before) return send(404, { error: "ResourceNotFound" });
    released.push(b.activation_id);
    return send(204);
  }
  if (req.headers.authorization !== `Bearer ${TOKEN}`) return send(401, { error: "Unauthorized" });
  if (req.method === "GET" && req.url === `/v1/license-keys/${KEY_ID}`) return send(200, { id: KEY_ID, activations });
  send(404, {});
});
await new Promise((r) => polar.listen(8788, r));

const env = { POLAR_API_BASE: "http://127.0.0.1:8788", POLAR_ACCESS_TOKEN: TOKEN, POLAR_ORGANIZATION_ID: ORG, POLAR_BENEFIT_IDS: BENEFIT };
let failures = 0;
const check = (name, ok, got) => { console.log(`${ok ? "PASS" : "FAIL"}  ${name}${ok ? "" : "  got: " + JSON.stringify(got)}`); if (!ok) failures++; };

let r = await reclaim(env, KEY, "not-a-hash");
check("rejects a malformed machine hash", r.state === "error", r);
r = await reclaim({ ...env, POLAR_ACCESS_TOKEN: "" }, KEY, MINE);
check("unconfigured worker says so, releases nothing", r.state === "error" && released.length === 0, r);
r = await reclaim(env, "REVOKED-OR-UNKNOWN", MINE);
check("revoked/disabled/unknown key → inactive, nothing released", r.state === "inactive" && released.length === 0, r);
r = await reclaim(env, "OTHER-APP-KEY", MINE);
check("key for a benefit not in POLAR_BENEFIT_IDS → none", r.state === "none" && released.length === 0, r);
r = await reclaim(env, KEY, "c".repeat(64));
check("computer with no activation on this key → none (really another computer)", r.state === "none" && released.length === 0, r);
r = await reclaim({ ...env, POLAR_ACCESS_TOKEN: "polar_oat_wrong" }, KEY, MINE);
check("bad token → names the missing scope, releases nothing", r.state === "error" && r.message.includes("license_keys:read") && released.length === 0, r);
r = await reclaim(env, KEY, MINE.toUpperCase());
check("this computer → releases ONLY its own activation", r.state === "reclaimed" && released.join() === "act-mine" && activations.map((a) => a.id).join() === "act-other", { r, released, activations });

const route = await (await worker.fetch(new Request("https://w.example/api/reclaim", { method: "POST", body: JSON.stringify({ key: KEY, machine: "d".repeat(64) }) }), env)).json();
check("POST /api/reclaim route", route.state === "none", route);
const health = await worker.fetch(new Request("https://w.example/"), env);
check("GET / health check", health.status === 200 && (await health.json()).ok === true, health.status);
const missing = await worker.fetch(new Request("https://w.example/other"), env);
check("unknown path → 404", missing.status === 404, missing.status);

polar.close();
console.log(failures ? `\n${failures} FAILED` : "\nALL PASSED");
process.exit(failures ? 1 : 0);
