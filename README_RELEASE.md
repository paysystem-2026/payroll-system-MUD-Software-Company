# Payroll System Release Pipeline

## Targets

- Windows x64: NSIS `.exe` + MSI `.msi`
- Windows x86: NSIS `.exe` + MSI `.msi` when supported by the Windows runner/toolchain
- Linux x64: AppImage + Debian `.deb` + RPM `.rpm`
- Signed Tauri updater artifacts and `latest.json`

## GitHub Secrets

Required:
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

The public updater key is intentionally stored in the workflow; it is not a secret.

## Release

Push a version tag such as:

```bash
git tag app-v1.0.0
git push origin app-v1.0.0
```

GitHub Actions builds the Windows x64/x86 and Linux x64 artifacts, publishes the GitHub Release, and verifies the updater manifest and release assets.

## Update testing

For a real update test, first install an older signed release (for example `1.0.0`), then publish a higher version (for example `1.0.1`) using the same signing key. In the installed app use **Updates → Check for updates**. The app must validate the signature before download/install.
