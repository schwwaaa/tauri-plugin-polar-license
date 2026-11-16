#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────
#  deploy.sh — publish the reclaim Worker to Cloudflare with plain curl.
#
#  Usage (regular Mac Terminal, inside this folder):
#     ./deploy.sh                  publish (asks for secrets the first time)
#     ./deploy.sh --reset-secrets  re-enter the three secrets, then publish
#
#  What it does, in order:
#     1. Reads config.env (your non-secret settings).
#     2. Reads secrets.env — or, the first time, asks you for the two
#        secrets (typing is hidden) and saves them there, readable only by
#        your user account. They never go anywhere except Cloudflare.
#     3. Uploads src/worker.js + all settings to Cloudflare (one HTTPS PUT).
#     4. Turns on the free *.workers.dev web address for it.
#     5. Prints the Worker address for your app's .reclaim_url(...) and
#        checks the Worker answers.
#
#  Needs only: bash, curl, python3 (preinstalled on macOS and most Linux;
#  on Windows run it from Git Bash or WSL).
# ─────────────────────────────────────────────────────────────────────────
set -euo pipefail
cd "$(dirname "$0")"

CF_API_BASE="${CF_API_BASE:-https://api.cloudflare.com/client/v4}"
CONFIG="config.env"
SECRETS="secrets.env"

say()  { printf '\n\033[1m%s\033[0m\n' "$*"; }
fail() { printf '\n\033[31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

# ── 1. Settings ─────────────────────────────────────────────────────────
say "1/5  Reading config.env"
[ -f "$CONFIG" ] || fail "config.env not found. Run:  cp config.env.example config.env  — then fill it in."
# shellcheck disable=SC1090
source "$CONFIG"
for v in CF_ACCOUNT_ID WORKER_NAME POLAR_API_BASE POLAR_ORGANIZATION_ID POLAR_BENEFIT_IDS; do
  val="${!v:-}"
  [ -n "$val" ] || fail "config.env: $v is empty."
  case "$val" in *PASTE-*) fail "config.env: $v still has a PASTE-… placeholder.";; esac
done
echo "     Worker:   $WORKER_NAME"
echo "     Polar:    $POLAR_API_BASE"

# ── 2. Secrets ──────────────────────────────────────────────────────────
say "2/5  Secrets"
if [ ! -f "$SECRETS" ] || [ "${1:-}" = "--reset-secrets" ]; then
  echo "     You'll paste two secrets. Typing is hidden — paste, then press Return."
  read -rsp "     Cloudflare API token: " CF_API_TOKEN; echo
  read -rsp "     Polar access token (starts polar_oat_): " POLAR_ACCESS_TOKEN; echo
  [ -n "$CF_API_TOKEN" ] && [ -n "$POLAR_ACCESS_TOKEN" ] || fail "Both tokens are required."
  ( umask 077
    printf 'CF_API_TOKEN=%q\nPOLAR_ACCESS_TOKEN=%q\n' "$CF_API_TOKEN" "$POLAR_ACCESS_TOKEN" > "$SECRETS" )
  echo "     Saved to secrets.env (only your user account can read it)."
else
  echo "     Using saved secrets.env  (re-enter with: ./deploy.sh --reset-secrets)"
fi
# shellcheck disable=SC1090
source "$SECRETS"

AUTH="Authorization: Bearer $CF_API_TOKEN"
API="$CF_API_BASE/accounts/$CF_ACCOUNT_ID/workers"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT

# Prints Cloudflare's error messages from a JSON response, or nothing if it worked.
cf_errors() {
  python3 -c 'import sys,json
try: d=json.load(open(sys.argv[1]))
except Exception: print("(Cloudflare sent a non-JSON answer)"); sys.exit()
if not d.get("success"): print("; ".join("%s: %s" % (e.get("code"), e.get("message")) for e in d.get("errors", [])) or "unknown error")' "$1"
}

# ── 3. Upload ───────────────────────────────────────────────────────────
say "3/5  Uploading src/worker.js to Cloudflare"
# The settings travel as "bindings": plain_text for settings, secret_text
# for secrets (Cloudflare stores those encrypted and never shows them again).
# Built with python3 so quotes/special characters can't break the JSON.
export POLAR_API_BASE POLAR_ORGANIZATION_ID POLAR_BENEFIT_IDS POLAR_ACCESS_TOKEN
python3 - > "$TMP/metadata.json" <<'PY'
import json, os
plain = ["POLAR_API_BASE", "POLAR_ORGANIZATION_ID", "POLAR_BENEFIT_IDS"]
secret = ["POLAR_ACCESS_TOKEN"]
# Empty settings are left out entirely (e.g. no email yet).
bindings  = [{"type": "plain_text",  "name": n, "text": os.environ[n]} for n in plain if os.environ.get(n)]
bindings += [{"type": "secret_text", "name": n, "text": os.environ[n]} for n in secret if os.environ.get(n)]
print(json.dumps({"main_module": "worker.js", "compatibility_date": "2026-06-01", "bindings": bindings}))
PY

code=$(curl -sS -o "$TMP/upload.json" -w '%{http_code}' -X PUT "$API/scripts/$WORKER_NAME" \
  -H "$AUTH" \
  -F "metadata=<$TMP/metadata.json;type=application/json" \
  -F "worker.js=@src/worker.js;type=application/javascript+module") || fail "Couldn't reach Cloudflare. Check the internet connection."
err=$(cf_errors "$TMP/upload.json")
if [ "$code" != "200" ] || [ -n "$err" ]; then
  case "$code" in
    401|403) fail "Cloudflare refused the API token ($code: $err). Re-create it with the \"Edit Cloudflare Workers\" template, then: ./deploy.sh --reset-secrets" ;;
    404)     fail "Cloudflare doesn't know that account ID ($err). Check CF_ACCOUNT_ID in config.env." ;;
    *)       fail "Upload failed ($code): $err" ;;
  esac
fi
echo "     Uploaded."

# ── 4. Web address ──────────────────────────────────────────────────────
say "4/5  Turning on the workers.dev address"
curl -sS -o "$TMP/sub.json" -X GET "$API/subdomain" -H "$AUTH" >/dev/null || true
SUB=$(python3 -c 'import sys,json
try: print((json.load(open(sys.argv[1])).get("result") or {}).get("subdomain") or "")
except Exception: print("")' "$TMP/sub.json")
if [ -z "$SUB" ]; then
  fail "Your Cloudflare account has no workers.dev subdomain yet. In the browser: Cloudflare dashboard → Workers & Pages. It asks you to pick a subdomain the first time — pick one, then run ./deploy.sh again."
fi
curl -sS -o "$TMP/enable.json" -X POST "$API/scripts/$WORKER_NAME/subdomain" \
  -H "$AUTH" -H "Content-Type: application/json" --data '{"enabled":true}' >/dev/null || true
err=$(cf_errors "$TMP/enable.json")
if [ -n "$err" ]; then
  echo "     Couldn't switch it on automatically ($err)."
  echo "     Do it in the browser: Cloudflare dashboard → Workers & Pages → $WORKER_NAME → Settings → Domains & Routes → workers.dev → Enable."
else
  echo "     On."
fi

BASE="https://$WORKER_NAME.$SUB.workers.dev"

# ── 5. Results + check ─────────────────────────────────────────────────
say "5/5  Done"
cat <<EOF
     Worker address — put it in your app's plugin config:
         .reclaim_url("$BASE")
EOF

printf '\n     Checking the Worker answers'
ok=""
for _ in $(seq 1 15); do
  if [ "$(curl -s -o /dev/null -w '%{http_code}' "$BASE/" || true)" = "200" ]; then ok=1; break; fi
  printf '.'; sleep 2
done
echo
if [ -n "$ok" ]; then
  echo "     $BASE answers — live."
else
  echo "     Not answering yet. New workers.dev addresses can take a minute or two;"
  echo "     open $BASE in the browser shortly; it shows {\"ok\":true}."
fi
