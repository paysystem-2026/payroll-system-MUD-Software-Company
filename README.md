# paylol-system-gorilla-doctor

[![Open in Bolt](https://bolt.new/static/open-in-bolt.svg)](https://bolt.new/~/sb1-5ubcbvmx)

## Production updater configuration

The desktop updater is implemented with the Tauri updater engine and verifies signed update bundles before installation. Update service configuration is supplied at build time so the endpoint and signing key are never editable application data.

Set these environment variables when producing release builds:

- `PAYROLL_UPDATE_ENDPOINT` — HTTPS updater endpoint. For GitHub Releases, use `https://github.com/paysystem-2026/payroll-system-MUD-Software-Company/releases/latest/download/latest.json`. Custom endpoints may contain `{channel}` plus Tauri updater placeholders such as `{{target}}`, `{{arch}}`, and `{{current_version}}`.
- `PAYROLL_UPDATE_PUBKEY` — Tauri updater public signing key matching the private key used to sign release artifacts.

If either value is absent, the application intentionally reports the updater as unavailable and continues normally; it never fabricates an update.


### GitHub Releases

Publish Tauri updater artifacts and the generated `latest.json` to GitHub Releases. The application reads that signed static manifest; it does not use GitHub API credentials and does not download unsigned packages. Tauri validates the updater signature before installation.
