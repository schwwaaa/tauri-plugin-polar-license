# How it works

> **Source tags.** **[Docs]** = stated in Polar's or Cloudflare's documentation.
> **[Tested]** = observed in real sandbox/production testing (Nov 2026).
> **[Design]** = how this plugin behaves, by choice.

## The lifecycle

### 1. First launch: the gate

1. The app opens. There's no license file, so the **gate** (activation popup) shows. The main window/engine isn't started.
2. The customer pastes the key. The app calls **activate** with: key, organization ID, label (computer name), `meta.machine` (fingerprint).
3. Polar answers:
   - **200:** the app checks that the key's **benefit ID matches this app**. If not, it releases the activation immediately and refuses ("different product"). If yes, it saves `license.json` and starts the app.
   - **403 limit reached:** see 4.4.
   - **404:** "not recognized" (typo, wrong environment, or a dead key).

### 2. Every later launch

1. The app reads `license.json`. It must match this build's environment and benefit, and **this computer's fingerprint**. A copied file doesn't match on another computer.
2. If it matches, **the app starts immediately, with no network wait.**
3. In the background, the **launch check** validates the activation with Polar:
   - **valid:** "License confirmed." in the license panel (`getLaunchCheck()`);
   - **Polar says not found** (revoked, disabled, expired or released): the file is marked revoked. **The current session keeps running.** The gate appears at the next launch;
   - **offline, timeout, or anything unclear:** nothing changes.

### 3. Release this machine

In the license panel: **Release this machine**, then click again within 4 seconds to confirm (the drop-in UI's guard against stray clicks). The plugin releases the activation in Polar and deletes `license.json`; the drop-in UI then calls your `onReleased` (typically: quit). The key can now be activated on any computer.

### 4. "Limit reached"

Polar says "activation limit reached" whenever all slots are taken, **including by this computer's own old activation**. The app then:

1. **Worker configured:** it asks the Worker, which validates the key:
   - **key not valid** → *"This license key has been deactivated."*;
   - **valid, and an activation carries this computer's fingerprint** → the Worker releases it, and the app activates again automatically;
   - **valid, but no activation from this computer** → *"…already activated on another computer…"*.
2. **No Worker:** *"…already activated on another computer…"*, even if the slot is this computer's own (see [C7](caveats.md#c7-a-lost-license-file-blocks-the-same-computer)). The one exception: if this computer's saved license shows the key was revoked, the app says *"…deactivated."*

### 5. What customers see

| Situation | Message |
|---|---|
| Typo / unknown key | That license key wasn't recognized. Check that you pasted the whole key. |
| Another product's key | This license key is for a different product… |
| All slots used by other computers | This license key is already activated on another computer. Each key works on a limited number of computers at a time. To move it here, open the app on the other computer and click "Release this machine" in the License section, then try again. |
| Key revoked / disabled | This license key has been deactivated. |
| No internet at activation | Couldn't connect to the license server. Check your internet connection and try again. |

Every message ends with *"Need help? Contact <SUPPORT_CONTACT>."* Polar's raw replies go only to the developer log (`[polar-license] …` lines), never to customers.

---


## Walkthroughs

Activation limit **1** in all of these. "Polar shows" means the key's activation count.

### 5.1 Normal purchase and use

| Step | Customer sees | Polar shows |
|---|---|---|
| Buys, receives the key | — | key `granted`, 0 activations |
| Opens the app, pastes the key | gate closes, app runs | 1 activation, labelled "Mayas-MacBook-Pro" |
| Next day, opens the app on a plane | app runs immediately; panel: "License not checked this time (no connection)" | unchanged |
| Opens it at home | "License confirmed." | unchanged |

### 5.2 Moving to a new computer

| Step | Customer sees | Polar shows |
|---|---|---|
| On the new laptop, pastes the key | "already activated on another computer…" | 1 activation (old Mac) |
| On the old Mac: License → Release this machine ×2 | "Released. Quitting…" | 0 activations |
| On the new laptop, pastes the key again | app runs | 1 activation (new laptop) |

### 5.3 Someone shares their key

| Step | Who / sees | Polar shows |
|---|---|---|
| Friend pastes the shared key | "already activated on another computer…" | 1 activation (owner's) |
| Owner releases, friend activates | friend's app runs; the owner's app asks for a key from its next launch | 1 activation, **different label** |
| They keep swapping | each swap needs a release | the label keeps changing (see section 7) |

### 5.4 You disable a key, then re-enable it

| Step | Customer sees | Polar shows |
|---|---|---|
| You disable it | current session keeps running | status `disabled` |
| Next launch | app runs; panel: "This license has been deactivated…" | |
| Launch after that | gate appears; pasting the key while it's disabled is refused | |
| You re-enable it | pasting the key: app runs [Tested] | status `granted` |

### 5.5 You revoke a key (or refund with revoke)

| Step | Customer sees | Polar shows |
|---|---|---|
| You revoke it | session keeps running | status `revoked` |
| Next launch → launch after | panel warns → gate; pasting the key: "deactivated" | |
| You set it back to active, or rotate it | **still "deactivated"** [Tested] | the key is dead |
| To restore access | issue a **new key** (free product or 100% code) | new key, new activations |

---

