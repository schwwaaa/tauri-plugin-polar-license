# Support playbook

The tickets you will actually get, and what to do about each. Releasing an activation step by step: [Polar setup §10](polar-setup.md#10-releasing-a-customers-activation).

| Customer says | Likely cause | Action |
|---|---|---|
| "It says already activated on another computer" | They changed computers without releasing, or lost their app data ([C6](caveats.md#c6-activations-never-expire-on-their-own), [C7](caveats.md#c7-a-lost-license-file-blocks-the-same-computer)) | Have them release on the old computer; otherwise release the activation yourself (API / dashboard). With the Worker, the same-computer case fixes itself. |
| "It says my key isn't recognized" | Typo, or an incomplete paste | Ask them to paste the full key from the email. |
| "It says deactivated" | You disabled or revoked it, or it was refunded | Disabled → re-enable when resolved. Revoked → issue a new key if appropriate ([C4](caveats.md#c4-revoke-is-permanent-disable-is-reversible), [C8](caveats.md#c8-no-direct-key-creation)). |
| "I never got my key" | Email delivery ([C2](caveats.md#c2-sandbox-emails-only-reach-organization-members), [C9](caveats.md#c9-the-customer-portal-may-not-show-keys)) | Look up their order or key in Polar and send it to them. |
| "Can I use it on two computers?" | Limit 1 | Raise that key's limit (API), or explain the release step ([walkthrough 5.2](how-it-works.md#52-moving-to-a-new-computer)). |

---

