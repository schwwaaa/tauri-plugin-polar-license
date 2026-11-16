# Caveats and their impact on you

Every limitation and surprise a developer meets when licensing a desktop app with Polar and this plugin. Each entry: **what happens**, **impact on you**, **what to do**.

> **Source tags.** **[Docs]** = stated in Polar's or Cloudflare's documentation.
> **[Tested]** = observed in real sandbox/production testing (Nov 2026).
> **[Design]** = how this plugin behaves, by choice.

### C1. Sandbox and production are separate worlds

*Source: Docs + Tested*

- **What:** different organization IDs, benefit IDs and keys. A production key sent to the sandbox, or the reverse, is "not recognized". The test card `4242 4242 4242 4242` only works in the sandbox.
- **Impact:** the most common "nothing works" cause during testing.
- **Do:** keep both ID sets in config. Development builds say *"(Test build: using sandbox Polar.)"* in their messages; check that before debugging anything else.

### C2. Sandbox emails only reach organization members

*Source: Docs + Tested*

- **What:** the docs say sandbox emails go only to members of your organization. In testing, only the **owner** email ever received the key email. Other addresses, even after being added as members, got a receipt without the key.
- **Impact:** you can't fully test the customer's key-delivery experience in the sandbox.
- **Do:** test delivery once in production with a free product or a 100% code.

### C3. Every key needs an activation limit

*Source: Docs*

- **What:** Polar refuses activation on keys whose benefit has no activation limit.
- **Do:** always set one on the License Keys benefit.

### C4. Revoke is permanent; disable is reversible

*Source: Tested; Docs silent*

- **What:** the docs list `granted / revoked / disabled` but don't say what they mean or which changes are allowed. In testing, a revoked key stayed dead after being set back to active and after rotation. A disabled key worked again after re-enabling.
- **Impact:** a revoke can't be undone. A customer whose key was revoked by mistake needs a **new order** to get a new key.
- **Do:** **disable** for anything temporary or uncertain. **Revoke** only for final decisions (refunds, confirmed abuse).

### C5. Rotating changes the string only

*Source: Docs + Tested*

- **What:** a new key string; the same status and the same activations. Rotating a revoked key doesn't revive it. Rotating doesn't free slots.
- **Do:** rotate only when a key **string** has leaked.

### C6. Activations never expire on their own

*Source: Docs + Tested*

- **What:** an activation counts until it's released. Polar doesn't know that a computer was wiped, sold or broken.
- **Impact:** with a low limit, customers who change computers without releasing will contact you.
- **Do:** choose a limit that fits how your customers work. Know how to release activations ([Support playbook](support-playbook.md)).

### C7. A lost license file blocks the same computer

*Source: Design*

- **What:** the app proves "this slot is mine" with the activation ID in `license.json`. If the file is lost (app data deleted, reinstall), Polar still counts the old activation, and **without the Worker** the computer is told "another computer".
- **Impact:** a support ticket per case.
- **Do:** either release it manually ([Support playbook](support-playbook.md)), or run the Worker. With the Worker, recovery is automatic on all OSes after app-data loss and app reinstalls, and on macOS after an OS reinstall. On Windows and Linux, an OS reinstall creates a new machine ID, so it stays manual.

### C8. No direct key creation

*Source: Docs*

- **What:** no button or API call creates a key. Keys come only from orders.
- **Impact:** replacements, gifts and press copies all need an order.
- **Do:** keep a private free product with the same benefit, or a capped 100% code.

### C9. The customer portal may not show keys

*Source: Docs vs Tested*

- **What:** the docs say keys are visible to customers and that customers can deactivate activations and rotate keys there. In testing (Nov 2026), the portal showed only Overview, Orders, Usage and Billing, with no keys.
- **Impact:** don't tell customers "find your key in the portal" or "release it in the portal" until you've confirmed it works in your production account.
- **Do:** point customers to the key email, and to you, for releases.

### C10. Refund defaults differ between dashboard and API

*Source: Docs*

- **What:** dashboard refunds revoke the key by default; API refunds don't unless `revoke_benefits: true`.
- **Do:** if you automate refunds, send `revoke_benefits: true`.

### C11. Revocations reach the app only at launch, while online

*Source: Design*

- **What:** the launch check runs once per launch, in the background. A revoked key keeps the current session running, shows the gate at the *next* launch, and does nothing on a machine that never goes online.
- **Why:** so licensing can never interrupt a live show.
- **Impact:** a refunded customer can keep using an offline machine.

### C12. The first activation needs internet

*Source: Design*

- **What:** after that, the app works offline indefinitely.

### C13. Client-side protection can be bypassed

*Source: Design*

- **What:** the license file isn't cryptographically signed, and any app can be patched. A hand-made license file is caught by the launch check whenever that computer is online (Polar has no such activation).
- **Impact:** this deters casual sharing; it doesn't stop a determined cracker.
- **Do:** rely on the activation limit and on activation tracking ([Tracking activations](tracking-activations.md)).

### C14. Never put your access token in the app

*Source: Design*

- **What:** the app only uses Polar's **public** license endpoints. Reading activations needs the token, so that's what the Worker is for.

### C15. Never change these after shipping

*Source: Design*

- **What:** the app's **bundle identifier** decides where `license.json` lives. Changing it makes every customer's file "disappear" ([C7](#c7-a-lost-license-file-blocks-the-same-computer) for everyone). Changing the **organization ID** or **benefit ID** makes every saved license stop matching.

### C16. Machine identity can change

*Source: Design*

- **What:** a hardware change (e.g. a logic board), a Windows or Linux OS reinstall, or a VM clone changes the machine ID, so the computer counts as new.
- **Impact:** an occasional support release.

### C17. Activation management in the dashboard isn't documented

*Source: Docs gap*

- **What:** Polar's docs describe viewing and releasing activations through the **API** (`GET /v1/license-keys/{id}`, deactivate), not where the dashboard shows them.
- **Do:** confirm in your dashboard where a key's activations appear, before you need it for support.

### C18. The Worker's free-tier limits

*Source: Docs*

- **What:** Cloudflare's free plan allows 100,000 requests per day **per account** (shared by all your Workers) and 100 Workers per account. The Worker is only called on "limit reached".

### C19. macOS unsigned builds

*Source: Design*

- **What:** without an Apple Developer ID and notarization, users go to System Settings → Privacy & Security → **Open Anyway**. If macOS says the app **"is damaged"** instead, the build wasn't signed at all.
- **Do:** test a browser-downloaded build on a second Mac once.

---

