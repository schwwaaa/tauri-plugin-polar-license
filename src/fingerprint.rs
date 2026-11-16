//! Identifying "this computer" without sending anything personal.

use sha2::{Digest, Sha256};

/// Salt for the hash. Changing it would make every existing activation look
/// like a different computer, so it is fixed forever.
const SALT: &[u8] = b"polar-license-v1:";

/// A stable, anonymous ID for this computer: SHA-256 of the OS machine ID.
///
/// | OS      | Source                        | Survives OS reinstall |
/// |---------|-------------------------------|-----------------------|
/// | macOS   | IOPlatformUUID (hardware)     | yes                   |
/// | Windows | registry `MachineGuid`        | no                    |
/// | Linux   | `/etc/machine-id`             | no                    |
///
/// The raw ID never leaves the computer. The hash is stored in the license
/// file (so a copied file doesn't work elsewhere) and in the activation's
/// `meta.machine` in Polar (so the reclaim Worker can recognize this
/// computer's own activations).
pub fn machine_fingerprint() -> String {
    let raw = machine_uid::get().unwrap_or_else(|_| "unknown-machine".into());
    hash_machine_id(&raw)
}

/// The hashing step on its own (exposed for tests and tooling).
pub fn hash_machine_id(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(SALT);
    h.update(raw.as_bytes());
    format!("{:x}", h.finalize())
}

/// The computer's name, recorded as the activation's label in Polar so you
/// can recognize it (e.g. `Mayas-MacBook-Pro`). Not unique; for display only.
pub fn machine_label() -> String {
    let host = gethostname::gethostname().to_string_lossy().to_string();
    let host = host.trim_end_matches(".local").trim();
    let host = if host.is_empty() {
        "Unnamed computer"
    } else {
        host
    };
    host.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_and_hex() {
        let a = machine_fingerprint();
        assert_eq!(a, machine_fingerprint());
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_format_is_frozen() {
        // Guards against accidental changes to SALT or the hash format, which
        // would orphan every existing activation. Expected value computed
        // independently: sha256("polar-license-v1:example-machine").
        assert_eq!(
            hash_machine_id("example-machine"),
            "7c682233448cbab67741a5cc418a3bedac9d05d8316542793b11b963db6440bb"
        );
    }
}
