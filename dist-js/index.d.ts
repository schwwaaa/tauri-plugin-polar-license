/** This computer's license state. */
export interface LicenseStatus {
    /** A valid license for this environment, product and computer is saved. */
    licensed: boolean;
    /** false while the Polar IDs are missing or placeholders. */
    configured: boolean;
    /** "production" or "sandbox". */
    environment: string;
    /** This computer's name as recorded in Polar. */
    label: string | null;
    /** Masked key, e.g. "****-E304DA". */
    displayKey: string | null;
    /** When the activation was created (ISO 8601). */
    activatedAt: string | null;
}
export type LaunchState = 'pending' | 'notLicensed' | 'confirmed' | 'deactivated' | 'offline';
/** Result of the background check that runs at each launch. */
export interface LaunchCheck {
    state: LaunchState;
    /** Customer-ready sentence describing the result. */
    message: string;
}
export type ErrorKind = 'notConfigured' | 'emptyKey' | 'notRecognized' | 'wrongProduct' | 'inUseElsewhere' | 'deactivated' | 'network' | 'storage' | 'nothingToRelease' | 'unexpected';
/** What `activate` / `release` reject with. `message` is customer-ready. */
export interface LicenseError {
    kind: ErrorKind;
    message: string;
}
/** Reads the saved license (local, instant, works offline). */
export declare function getStatus(): Promise<LicenseStatus>;
/**
 * Activates a key on this computer. On success the plugin runs your
 * `on_licensed` hook (e.g. opens the main window) before resolving.
 * Rejects with a {@link LicenseError}.
 */
export declare function activate(key: string): Promise<LicenseStatus>;
/**
 * Frees this computer's activation so the key can be used elsewhere, and
 * deletes the local license. What happens next (usually quitting) is up to
 * the app. Rejects with a {@link LicenseError}.
 */
export declare function release(): Promise<void>;
/** Result of this launch's background check ("pending" while it runs). */
export declare function getLaunchCheck(): Promise<LaunchCheck>;
/** Polls {@link getLaunchCheck} until it leaves "pending" (or times out). */
export declare function waitForLaunchCheck(timeoutMs?: number, intervalMs?: number): Promise<LaunchCheck>;
