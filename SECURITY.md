# Security Policy

## Supported Versions

Only the latest tagged release receives security fixes.

## Reporting a Vulnerability

Please report vulnerabilities privately via
[GitHub Security Advisories](https://github.com/flaukowski/kannaka-crystal/security/advisories/new).
Do not open a public issue for exploitable bugs. You should receive a
response within a week.

## Scope Notes

- The REST API binds to `127.0.0.1` by default and has **no authentication**
  — it is a local research instrument. Do not expose it to untrusted
  networks; if you bind it publicly, put an authenticating reverse proxy in
  front of it.
- NATS credentials are read only from `KANNAKA_NATS_CREDS` /
  `KANNAKA_NATS_URL` environment variables. Never commit credentials,
  `.creds` files, or `.env` files to the repository.
- `.crystal` programs are data, not code: the interpreter has no
  filesystem, network, or process primitives.
- Dependencies are audited in CI with `cargo audit` (see
  `.github/workflows/security.yml`).
