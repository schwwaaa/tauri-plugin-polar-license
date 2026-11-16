/**
 * Front-end bindings for tauri-plugin-polar-license.
 *
 * ```ts
 * import { getStatus, activate, release, getLaunchCheck } from 'tauri-plugin-polar-license-api';
 *
 * const status = await getStatus();
 * if (!status.licensed) {
 *   try {
 *     await activate(keyFromInput);   // starts your app via on_licensed
 *   } catch (e) {
 *     const err = e as LicenseError;  // { kind, message }
 *     showError(err.message);         // customer-ready wording
 *   }
 * }
 * ```
 *
 * The app needs the `polar-license:default` permission in a capability.
 */
import { invoke } from '@tauri-apps/api/core';
/** Reads the saved license (local, instant, works offline). */
export async function getStatus() {
    return invoke('plugin:polar-license|status');
}
/**
 * Activates a key on this computer. On success the plugin runs your
 * `on_licensed` hook (e.g. opens the main window) before resolving.
 * Rejects with a {@link LicenseError}.
 */
export async function activate(key) {
    return invoke('plugin:polar-license|activate', { key });
}
/**
 * Frees this computer's activation so the key can be used elsewhere, and
 * deletes the local license. What happens next (usually quitting) is up to
 * the app. Rejects with a {@link LicenseError}.
 */
export async function release() {
    return invoke('plugin:polar-license|release');
}
/** Result of this launch's background check ("pending" while it runs). */
export async function getLaunchCheck() {
    return invoke('plugin:polar-license|launch_check');
}
/** Polls {@link getLaunchCheck} until it leaves "pending" (or times out). */
export async function waitForLaunchCheck(timeoutMs = 30000, intervalMs = 1000) {
    const started = Date.now();
    for (;;) {
        const check = await getLaunchCheck();
        if (check.state !== 'pending' || Date.now() - started > timeoutMs)
            return check;
        await new Promise((r) => setTimeout(r, intervalMs));
    }
}
