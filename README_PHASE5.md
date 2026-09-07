# LAN Transfer - Phase 5

Phase 5 adds safe LAN-backup import protection without changing the existing pairing/encryption/transfer protocol.

## Added
- Received LAN backups can be imported from the LAN Transfer page.
- A `pre_import` encrypted safety backup is created before import.
- The incoming backup is validated before database replacement.
- Database migrations/version upgrade run after replacement.
- If any import/upgrade/finalization step fails, the exact pre-import database snapshot is restored automatically.
- The safety backup remains in Backup & Restore history after a successful import or rollback.
- Existing backup settings are preserved across import.
- Existing authenticated session is cleared after a successful import so the restored admin database is not used with an old session.
- Existing LAN pairing, AES-GCM transfer encryption, authentication tags, checksum verification, and trusted-device controls remain unchanged.

## Verification on the development machine
Run:

```bash
npm install
npm run build
npx tauri dev
```

Then test: send backup -> receive -> LAN Transfer / Received LAN Backups -> Import Safely.
