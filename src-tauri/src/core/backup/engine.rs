use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use rand::RngCore;
use rusqlite::{backup::Backup, params, Connection};
use sha2::{Digest, Sha256};
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::{SystemTime, UNIX_EPOCH}};

use crate::database::migrations;
use super::retention;

const MAGIC: &[u8] = b"PSBK1";
const NONCE_LEN: usize = 12;
const KEY_FILE: &str = "backup.key";

// Serialize backup/restore file operations inside this process. This prevents
// a scheduled backup from racing with a manual backup or restore.
static BACKUP_RUN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn backup_run_lock() -> &'static Mutex<()> {
    BACKUP_RUN_LOCK.get_or_init(|| Mutex::new(()))
}

fn acquire_backup_lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    backup_run_lock().lock().map_err(|_| "Backup service is unavailable because its lock was poisoned.".to_string())
}

pub fn app_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".payroll-system")
}

pub fn default_backup_dir() -> PathBuf { app_dir().join("backups") }

fn now_id() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn key_path() -> PathBuf { app_dir().join(KEY_FILE) }

fn load_key() -> Result<[u8; 32], String> {
    fs::create_dir_all(app_dir()).map_err(|e| format!("Unable to prepare security storage: {e}"))?;
    let path = key_path();
    if path.exists() {
        let bytes = fs::read(&path).map_err(|e| format!("Unable to read backup key: {e}"))?;
        if bytes.len() == 32 { let mut key = [0u8; 32]; key.copy_from_slice(&bytes); return Ok(key); }
        return Err("Backup encryption key is invalid.".into());
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let mut file = fs::OpenOptions::new().create_new(true).write(true).open(&path).map_err(|e| format!("Unable to create backup key: {e}"))?;
    file.write_all(&key).map_err(|e| format!("Unable to store backup key: {e}"))?;
    file.sync_all().ok();
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

fn encrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    let key = load_key()?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce_bytes), data).map_err(|_| "Backup encryption failed.".to_string())?;
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() <= MAGIC.len() + NONCE_LEN || &data[..MAGIC.len()] != MAGIC { return Err("Invalid or unsupported backup file.".into()); }
    let key = load_key()?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce_start = MAGIC.len();
    let nonce_end = nonce_start + NONCE_LEN;
    cipher.decrypt(Nonce::from_slice(&data[nonce_start..nonce_end]), &data[nonce_end..]).map_err(|_| "Backup integrity check failed or the backup belongs to another installation.".to_string())
}

fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| format!("Compression failed: {e}"))?;
    encoder.finish().map_err(|e| format!("Compression failed: {e}"))
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|e| format!("Backup decompression failed: {e}"))?;
    Ok(out)
}

fn checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn database_path() -> PathBuf { app_dir().join("payroll.db") }

fn setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get(0)).ok()
}

fn expand_location(value: &str) -> PathBuf {
    if value == "~" { return PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| value.to_string())); }
    if let Some(rest) = value.strip_prefix("~/") {
        return PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(rest);
    }
    PathBuf::from(value)
}

pub fn backup_settings(conn: &Connection) -> (bool, String, String, i64, String) {
    let enabled = setting(conn, "backup_enabled").unwrap_or_else(|| "1".into()) != "0";
    let frequency = match setting(conn, "backup_frequency").as_deref() {
        Some("weekly") => "weekly".to_string(),
        _ => "daily".to_string(),
    };
    let time = setting(conn, "backup_time")
        .filter(|value| {
            let bytes = value.as_bytes();
            if bytes.len() != 5 || bytes[2] != b':' { return false; }
            let hour = std::str::from_utf8(&bytes[0..2]).ok().and_then(|v| v.parse::<u8>().ok());
            let minute = std::str::from_utf8(&bytes[3..5]).ok().and_then(|v| v.parse::<u8>().ok());
            matches!((hour, minute), (Some(h), Some(m)) if h < 24 && m < 60)
        })
        .unwrap_or_else(|| "02:00".into());
    let retention = setting(conn, "backup_retention").and_then(|v| v.parse().ok()).unwrap_or(7).clamp(1, 365);
    let location = setting(conn, "backup_location").filter(|v| !v.trim().is_empty()).map(|v| expand_location(&v)).unwrap_or_else(default_backup_dir).to_string_lossy().to_string();
    (enabled, frequency, time, retention, location)
}

fn database_version(conn: &Connection) -> String {
    conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| r.get::<_, i64>(0)).unwrap_or(0).to_string()
}

fn app_version(conn: &Connection) -> String {
    setting(conn, "app_version").unwrap_or_else(|| "1.0.0".into())
}

fn validate_database_file(path: &Path) -> Result<(), String> {
    let check = Connection::open(path)
        .and_then(|c| c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0)))
        .map_err(|e| format!("Backup database is invalid: {e}"))?;
    if check != "ok" { return Err("Backup integrity check failed.".into()); }
    Ok(())
}

fn payload_from_file(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Unable to read backup file: {e}"))?;
    if bytes.starts_with(MAGIC) {
        return decompress(&decrypt(&bytes)?);
    }
    // Backward compatibility: allow an explicitly selected legacy SQLite backup.
    if bytes.starts_with(b"SQLite format 3\0") {
        return Ok(bytes);
    }
    Err("This file is not a supported Payroll System backup (.pbak or legacy SQLite backup).".into())
}

pub fn create_backup(conn: &Connection, backup_dir: &Path, backup_type: &str) -> Result<(String, u64, String), String> {
    let _guard = acquire_backup_lock()?;
    create_backup_locked(conn, backup_dir, backup_type)
}

fn create_backup_locked(conn: &Connection, backup_dir: &Path, backup_type: &str) -> Result<(String, u64, String), String> {
    fs::create_dir_all(backup_dir).map_err(|e| format!("Unable to create backup folder: {e}"))?;
    let temp = app_dir().join(format!("backup_snapshot_{}.sqlite", now_id()));
    let mut snapshot = Connection::open(&temp).map_err(|e| format!("Unable to create backup snapshot: {e}"))?;
    {
        let backup = Backup::new(conn, &mut snapshot).map_err(|e| format!("Unable to snapshot database: {e}"))?;
        backup.run_to_completion(5, std::time::Duration::from_millis(20), None).map_err(|e| format!("Unable to snapshot database: {e}"))?;
    }
    validate_database_file(&temp)?;
    drop(snapshot);
    let raw = fs::read(&temp).map_err(|e| format!("Unable to read snapshot: {e}"))?;
    let _ = fs::remove_file(&temp);
    let compressed = compress(&raw)?;
    let encrypted = encrypt(&compressed)?;
    let name = format!("payroll_backup_{}_{}.pbak", backup_type, now_id());
    let path = backup_dir.join(name);
    let mut file = fs::File::create(&path).map_err(|e| format!("Unable to create backup file: {e}"))?;
    file.write_all(&encrypted).map_err(|e| format!("Unable to write backup file: {e}"))?;
    file.sync_all().map_err(|e| format!("Unable to finalize backup file: {e}"))?;
    let size = encrypted.len() as u64;
    let sum = checksum(&encrypted);
    conn.execute("INSERT INTO backups (file_path,file_size,backup_type,status,checksum,encrypted,database_version,app_version) VALUES (?1,?2,?3,'completed',?4,1,?5,?6)", params![path.to_string_lossy().to_string(), size as i64, backup_type, sum, database_version(conn), app_version(conn)]).map_err(|e| format!("Backup history error: {e}"))?;
    if backup_type == "automatic" {
        let retention_count = setting(conn, "backup_retention").and_then(|v| v.parse().ok()).unwrap_or(7);
        retention::apply(conn, retention_count);
    }
    Ok((path.to_string_lossy().to_string(), size, sum))
}

fn record(conn: &Connection, id: i64) -> Result<(String, String), String> {
    conn.query_row("SELECT file_path, status FROM backups WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?))).map_err(|_| "Backup record not found.".to_string())
}

// Resolve a backup path safely when the application was moved, upgraded, or the
// backup folder was changed. We only fall back to the filename inside the
// currently configured backup directory; we never search arbitrary locations.
fn resolve_backup_path(conn: &Connection, stored_path: &str) -> PathBuf {
    let stored = PathBuf::from(stored_path);
    if stored.exists() {
        return stored;
    }
    if let Some(name) = stored.file_name() {
        let (_, _, _, _, location) = backup_settings(conn);
        let candidate = PathBuf::from(location).join(name);
        if candidate.exists() {
            return candidate;
        }
    }
    stored
}


/// Read and decrypt a completed backup for secure LAN transport. The LAN layer
/// re-encrypts the database with the receiving installation's own backup key.
pub fn read_lan_payload(conn: &Connection, id: i64) -> Result<(Vec<u8>, String, String, String), String> {
    verify_backup(conn, id)?;
    let (stored_path, status) = record(conn, id)?;
    if status != "completed" { return Err("Backup is not completed.".into()); }
    let path = resolve_backup_path(conn, &stored_path);
    let payload = payload_from_file(&path)?;
    let filename = path.file_name().and_then(|v| v.to_str()).unwrap_or("payroll_backup.pbak").to_string();
    Ok((payload, filename, database_version(conn), app_version(conn)))
}

/// Validate an incoming SQLite payload and store it as a new backup encrypted
/// with the receiving installation's local backup key.
pub fn store_lan_received_payload(
    conn: &Connection,
    backup_dir: &Path,
    filename: &str,
    payload: &[u8],
    database_version_value: &str,
    app_version_value: &str,
) -> Result<(i64, String, u64, String), String> {
    let _guard = acquire_backup_lock()?;
    validate_database_bytes(payload)?;
    fs::create_dir_all(backup_dir).map_err(|e| format!("Unable to create backup folder: {e}"))?;
    let safe_name = Path::new(filename).file_name().and_then(|v| v.to_str()).unwrap_or("received-backup")
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_', "_");
    let final_path = backup_dir.join(format!("payroll_backup_lan_received_{}_{}.pbak", now_id(), safe_name));
    let compressed = compress(payload)?;
    let encrypted = encrypt(&compressed)?;
    let mut file = fs::File::create(&final_path).map_err(|e| format!("Unable to create received backup: {e}"))?;
    file.write_all(&encrypted).map_err(|e| format!("Unable to write received backup: {e}"))?;
    file.sync_all().map_err(|e| format!("Unable to finalize received backup: {e}"))?;
    let size = encrypted.len() as u64;
    let sum = checksum(&encrypted);
    conn.execute(
        "INSERT INTO backups (file_path,file_size,backup_type,status,checksum,encrypted,database_version,app_version) VALUES (?1,?2,'lan_received','completed',?3,1,?4,?5)",
        params![final_path.to_string_lossy().to_string(), size as i64, sum, database_version_value, app_version_value],
    ).map_err(|e| { let _ = fs::remove_file(&final_path); format!("Unable to record received backup: {e}") })?;
    let id = conn.last_insert_rowid();
    Ok((id, final_path.to_string_lossy().to_string(), size, sum))
}

fn validate_database_bytes(payload: &[u8]) -> Result<(), String> {
    let temp = app_dir().join(format!("lan_validate_{}.sqlite", now_id()));
    fs::write(&temp, payload).map_err(|e| format!("Unable to prepare received backup validation: {e}"))?;
    let result = validate_database_file(&temp);
    let _ = fs::remove_file(&temp);
    result
}

pub fn verify_backup(conn: &Connection, id: i64) -> Result<(), String> {
    let _guard = acquire_backup_lock()?;
    let (stored_path, _) = record(conn, id)?;
    let path = resolve_backup_path(conn, &stored_path);
    let bytes = fs::read(&path).map_err(|e| format!("Unable to read backup file '{}': {e}", path.display()))?;
    let expected = conn.query_row("SELECT checksum FROM backups WHERE id = ?1", [id], |r| r.get::<_, Option<String>>(0)).ok().flatten();
    if let Some(expected) = expected {
        let actual = checksum(&bytes);
        if actual != expected { return Err("Backup checksum does not match. The file may have been changed or damaged.".into()); }
    }
    let payload = if bytes.starts_with(MAGIC) { decompress(&decrypt(&bytes)?)? } else if bytes.starts_with(b"SQLite format 3\0") { bytes } else { return Err("Unsupported backup format.".into()) };
    let temp = app_dir().join(format!("verify_{}.sqlite", now_id()));
    fs::write(&temp, &payload).map_err(|e| format!("Unable to prepare verification: {e}"))?;
    let result = validate_database_file(&temp);
    let _ = fs::remove_file(&temp);
    result
}

fn write_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES (?1,?2,datetime('now'))", params![key, value]).map_err(|e| e.to_string()).map(|_| ())
}

fn restore_payload(conn: &mut Connection, payload: &[u8], source_label: &str, safety_type: &str) -> Result<(), String> {
    let _guard = acquire_backup_lock()?;
    let source_path = app_dir().join(format!("restore_{}.sqlite", now_id()));
    fs::write(&source_path, payload).map_err(|e| format!("Unable to prepare restore: {e}"))?;
    validate_database_file(&source_path)?;
    let source = Connection::open(&source_path).map_err(|e| format!("Unable to open backup database: {e}"))?;
    source.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| format!("Unable to configure restore database: {e}"))?;

    let has_admin: bool = source.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='admin_users'", [], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0;
    let has_settings: bool = source.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_settings'", [], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0;
    if !has_admin || !has_settings {
        let _ = fs::remove_file(&source_path);
        return Err(format!("The selected file is not a valid Payroll System database ({source_label})."));
    }

    // Snapshot the live database BEFORE replacing it. This is the rollback point
    // if any restore/import step fails after the database has started changing.
    let (_, current_frequency, current_time, current_retention, current_location) = backup_settings(conn);
    let current_enabled = setting(conn, "backup_enabled").unwrap_or_else(|| "1".into());
    let current_backup = create_backup_locked(conn, Path::new(&current_location), safety_type)?;
    let current_db_version = database_version(conn);
    let current_app_version = app_version(conn);

    let apply_result = (|| {
        conn.execute_batch("PRAGMA foreign_keys=OFF;")
            .map_err(|e| format!("Unable to prepare restore: {e}"))?;
        {
            let backup = Backup::new(&source, conn).map_err(|e| format!("Unable to restore database: {e}"))?;
            backup.run_to_completion(5, std::time::Duration::from_millis(20), None)
                .map_err(|e| format!("Unable to restore database: {e}"))?;
        }
        migrations::run(conn).map_err(|e| format!("Database upgrade after restore failed: {e}"))?;
        write_setting(conn, "backup_enabled", &current_enabled)?;
        write_setting(conn, "backup_frequency", &current_frequency)?;
        write_setting(conn, "backup_time", &current_time)?;
        write_setting(conn, "backup_retention", &current_retention.to_string())?;
        write_setting(conn, "backup_location", &current_location)?;
        write_setting(conn, "app_version", &current_app_version)?;
        conn.execute("DELETE FROM admin_sessions", []).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| format!("Unable to finalize restored database: {e}"))?;
        // Persist the safety point inside the restored database. If this fails,
        // the whole operation is treated as failed and automatic rollback runs.
        conn.execute("INSERT INTO backups (file_path,file_size,backup_type,status,checksum,encrypted,database_version,app_version) VALUES (?1,?2,?3,'completed',?4,1,?5,?6)",
            params![current_backup.0, current_backup.1 as i64, safety_type, current_backup.2, current_db_version, current_app_version])
            .map_err(|e| format!("Unable to preserve safety backup history: {e}"))?;
        Ok::<(), String>(())
    })();

    let result = match apply_result {
        Ok(()) => Ok(()),
        Err(original_error) => {
            // The import/restore failed. Restore the exact pre-operation snapshot
            // before reporting failure; this prevents a half-imported database.
            let rollback = (|| -> Result<(), String> {
                let safety_payload = payload_from_file(Path::new(&current_backup.0))?;
                let safety_path = app_dir().join(format!("rollback_{}.sqlite", now_id()));
                fs::write(&safety_path, &safety_payload).map_err(|e| format!("Unable to prepare rollback: {e}"))?;
                validate_database_file(&safety_path)?;
                let safety_source = Connection::open(&safety_path).map_err(|e| format!("Unable to open rollback snapshot: {e}"))?;
                {
                    let backup = Backup::new(&safety_source, conn).map_err(|e| format!("Unable to rollback database: {e}"))?;
                    backup.run_to_completion(5, std::time::Duration::from_millis(20), None)
                        .map_err(|e| format!("Unable to rollback database: {e}"))?;
                }
                migrations::run(conn).map_err(|e| format!("Database upgrade after rollback failed: {e}"))?;
                write_setting(conn, "backup_enabled", &current_enabled)?;
                write_setting(conn, "backup_frequency", &current_frequency)?;
                write_setting(conn, "backup_time", &current_time)?;
                write_setting(conn, "backup_retention", &current_retention.to_string())?;
                write_setting(conn, "backup_location", &current_location)?;
                write_setting(conn, "app_version", &current_app_version)?;
                conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| format!("Unable to finalize rollback database: {e}"))?;
                conn.execute("INSERT INTO backups (file_path,file_size,backup_type,status,checksum,encrypted,database_version,app_version) VALUES (?1,?2,?3,'completed',?4,1,?5,?6)",
                    params![current_backup.0, current_backup.1 as i64, safety_type, current_backup.2, current_db_version, current_app_version])
                    .map_err(|e| format!("Unable to preserve rollback backup: {e}"))?;
                let _ = fs::remove_file(&safety_path);
                Ok(())
            })();
            match rollback {
                Ok(()) => Err(format!("Restore/import failed and the original database was safely restored: {original_error}")),
                Err(rollback_error) => Err(format!("Restore/import failed and automatic rollback also failed. Do not continue using the database until recovery is completed. Original error: {original_error}; rollback error: {rollback_error}")),
            }
        }
    };

    drop(source);
    let _ = fs::remove_file(&source_path);
    result
}

pub fn import_lan_backup(conn: &mut Connection, id: i64) -> Result<(), String> {
    let (stored_path, status): (String, String) = conn.query_row(
        "SELECT file_path, status FROM backups WHERE id = ?1 AND backup_type = 'lan_received'",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Received LAN backup not found.".to_string())?;
    if status != "completed" { return Err("The received LAN backup is not ready for import.".into()); }
    let path = resolve_backup_path(conn, &stored_path);
    let payload = payload_from_file(&path)?;
    restore_payload(conn, &payload, &path.to_string_lossy(), "pre_import")
}

pub fn restore_backup(conn: &mut Connection, id: i64) -> Result<(), String> {
    let (stored_path, _) = record(conn, id)?;
    let path = resolve_backup_path(conn, &stored_path);
    let payload = payload_from_file(&path)?;
    restore_payload(conn, &payload, &path.to_string_lossy(), "pre_restore")
}

pub fn restore_backup_file(conn: &mut Connection, path: &Path) -> Result<(), String> {
    if !path.exists() { return Err("The selected backup file no longer exists.".into()); }
    let payload = payload_from_file(path)?;
    restore_payload(conn, &payload, &path.to_string_lossy(), "pre_restore")
}

pub fn delete_backup(conn: &Connection, id: i64) -> Result<(), String> {
    let _guard = acquire_backup_lock()?;
    let (stored_path, _) = record(conn, id)?;
    if stored_path.starts_with("preview://") {
        conn.execute("DELETE FROM backups WHERE id = ?1", [id]).map_err(|e| format!("Unable to delete backup history: {e}"))?;
        return Ok(());
    }
    let path = resolve_backup_path(conn, &stored_path);
    // The history row is authoritative. If an old file was already removed,
    // deleting the history entry should still succeed instead of leaving a
    // permanently stuck backup row in the UI.
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Unable to delete backup file '{}': {e}", path.display()))?;
    }
    conn.execute("DELETE FROM backups WHERE id = ?1", [id]).map_err(|e| format!("Unable to delete backup history: {e}"))?;
    Ok(())
}

pub fn db_path_for_scheduler() -> PathBuf { database_path() }

// The scheduler passes its already-open SQLite connection here. This avoids
// opening a second connection to the same WAL database and makes automatic
// backups behave consistently with manual backups.
pub fn run_automatic_backup_with_connection(conn: &Connection) -> Result<(), String> {
    let (enabled, _frequency, _time, _retention, location) = backup_settings(conn);
    if !enabled { return Ok(()); }
    match create_backup(conn, Path::new(&location), "automatic") {
        Ok(_) => {
            let id = conn.query_row("SELECT id FROM backups WHERE backup_type='automatic' ORDER BY id DESC LIMIT 1", [], |r| r.get::<_, i64>(0)).ok();
            crate::security::audit::log(conn, "automatic_backup_created", Some("backups"), id, Some("Scheduled automatic backup created and verified"));
            Ok(())
        }
        Err(e) => {
            crate::security::audit::log(conn, "automatic_backup_failed", Some("backups"), None, Some(&e));
            Err(e)
        }
    }
}

pub fn run_automatic_backup() -> Result<(), String> {
    let db_path = database_path();
    if !db_path.exists() { return Ok(()); }
    let conn = Connection::open(&db_path).map_err(|e| format!("Scheduler database error: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| format!("Scheduler database configuration error: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").ok();
    run_automatic_backup_with_connection(&conn)
}
