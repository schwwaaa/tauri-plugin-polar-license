# Security policy

## Reporting a vulnerability

Please **don't open a public issue**. Use GitHub's private vulnerability
reporting on this repository (Security → Report a vulnerability). You'll get
a response within a few days.

## Scope and threat model

In scope:
- Anything that lets an app accept a key or license it shouldn't: a wrong product, wrong environment, revoked key, or another computer's license file.
- Anything that makes a licensed launch fail or block when it shouldn't.
- Leaks of keys, tokens or machine identifiers beyond what the docs describe.
- The reclaim Worker releasing an activation that doesn't belong to the requesting computer.

Known and documented (not vulnerabilities):
- Client-side checks can be bypassed by patching the app binary.
- The local license file isn't cryptographically signed; a hand-made file is caught by the launch check whenever the computer is online.
- A revoked key keeps working on a computer that never goes online again (by design: launches never block).

See [docs/caveats.md](docs/caveats.md) (C11, C13, C14).
