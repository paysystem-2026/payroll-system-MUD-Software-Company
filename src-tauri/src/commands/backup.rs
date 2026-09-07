use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::connection::Database;
use crate::security::session;
use crate::security::audit;
use crate::core::backup;

#[derive(Serialize)]
pub struct BackupRecord {
    pub id: i64,
    pub file_path: String,
    pub file_size: i64,
    pub backup_type: String,
    pub status: String,
    pub created_at: String,
    pub checksum: Option<String>,
    pub encrypted: bool,
    pub database_version: Option<String>,
    pub app_version: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct BackupSettingsResponse {
    pub enabled: bool,
    pub frequency: String,
    pub time: String,
    pub retention: i64,
    pub location: String,
}

#[derive(Serialize)]
pub struct BackupStatusResponse {
    pub last_backup: Option<BackupRecord>,
    pub next_backup: Option<String>,
    pub backup_count: i64,
    pub settings: BackupSettingsResponse,
}

#[derive(Serialize)]
pub struct BackupResponse { pub success: bool, pub message: String, pub backup: Option<BackupRecord> }

#[derive(Deserialize)]
pub struct BackupSettingsRequest { pub enabled: bool, pub frequency: String, pub time: String, pub retention: i64, pub location: String }

fn authorized(conn: &rusqlite::Connection, token: &str, allow_locked: bool) -> Result<i64, String> {
    // Reuse the same encrypted-session validation as the authentication commands.
    let mut stmt = conn.prepare("SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if let Ok(stored) = session::decrypt_token(&row.1) {
            if stored == token && (allow_locked || row.2 == 0) { return Ok(row.0); }
        }
    }
    Err("Invalid or locked session".into())
}

fn settings(conn: &rusqlite::Connection) -> BackupSettingsResponse {
    let (enabled, frequency, time, retention, location) = backup::backup_settings(conn);
    BackupSettingsResponse { enabled, frequency, time, retention, location }
}

fn row(conn: &rusqlite::Connection, id: i64) -> Option<BackupRecord> {
    conn.query_row("SELECT id,file_path,file_size,backup_type,status,created_at,checksum,encrypted,database_version,app_version FROM backups WHERE id = ?1", [id], |r| Ok(BackupRecord {
        id: r.get(0)?, file_path: r.get(1)?, file_size: r.get::<_, Option<i64>>(2)?.unwrap_or(0), backup_type: r.get(3)?, status: r.get(4)?, created_at: r.get(5)?, checksum: r.get(6)?, encrypted: r.get::<_, i64>(7).unwrap_or(0) == 1, database_version: r.get(8)?, app_version: r.get(9)?
    })).ok()
}

#[tauri::command]
pub fn get_backup_settings(db: State<Database>, token: String) -> Result<BackupSettingsResponse, String> {
    let conn = db.conn.lock().unwrap(); authorized(&conn, &token, false)?; Ok(settings(&conn))
}

#[tauri::command]
pub fn update_backup_settings(db: State<Database>, token: String, settings: BackupSettingsRequest) -> BackupResponse {
    let conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse { success:false, message:e, backup:None }; }
    let frequency = if settings.frequency == "weekly" { "weekly" } else { "daily" };
    let time = if settings.time.len() == 5 && settings.time.as_bytes().get(2) == Some(&b':') { settings.time.clone() } else { "02:00".into() };
    let retention = settings.retention.clamp(1, 365);
    let location = if settings.location.trim().is_empty() { backup::default_backup_dir().to_string_lossy().to_string() } else { settings.location.trim().to_string() };
    let expanded_location = if location == "~" {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
    } else if let Some(rest) = location.strip_prefix("~/") {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(rest)
    } else {
        std::path::PathBuf::from(&location)
    };
    if let Err(e) = std::fs::create_dir_all(&expanded_location) {
        return BackupResponse { success:false, message:format!("Unable to use backup location '{}': {e}", location), backup:None };
    }
    let result = conn.execute_batch("BEGIN IMMEDIATE;");
    let result = result.and_then(|_| conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('backup_enabled',?1,datetime('now'))", rusqlite::params![if settings.enabled { "1" } else { "0" }]));
    let result = result.and_then(|_| conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('backup_frequency',?1,datetime('now'))", rusqlite::params![frequency]));
    let result = result.and_then(|_| conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('backup_time',?1,datetime('now'))", rusqlite::params![time]));
    let result = result.and_then(|_| conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('backup_retention',?1,datetime('now'))", rusqlite::params![retention.to_string()]));
    let stored_location = expanded_location.to_string_lossy().to_string();
    let result = result.and_then(|_| conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('backup_location',?1,datetime('now'))", rusqlite::params![stored_location]));
    let result = result.and_then(|_| conn.execute_batch("COMMIT;"));
    match result {
        Ok(_) => { audit::log(&conn, "backup_settings_updated", Some("app_settings"), None, Some("Backup configuration updated")); BackupResponse { success:true, message:"Backup settings saved".into(), backup:None } },
        Err(e) => BackupResponse { success:false, message:e.to_string(), backup:None }
    }
}

#[tauri::command]
pub fn get_backup_status(db: State<Database>, token: String) -> Result<BackupStatusResponse, String> {
    let conn = db.conn.lock().unwrap(); authorized(&conn, &token, false)?;
    let last = conn.query_row("SELECT id FROM backups WHERE status='completed' ORDER BY id DESC LIMIT 1", [], |r| r.get::<_, i64>(0)).ok().and_then(|id| row(&conn,id));
    let count = conn.query_row("SELECT COUNT(*) FROM backups WHERE status='completed'", [], |r| r.get::<_, i64>(0)).unwrap_or(0);
    let (enabled, frequency, time, retention, location) = backup::backup_settings(&conn);
    let next = if enabled { Some(format!("{} at {}", if frequency == "weekly" { "Next scheduled backup" } else { "Next backup" }, time)) } else { None };
    Ok(BackupStatusResponse { last_backup:last, next_backup:next, backup_count:count, settings:BackupSettingsResponse{enabled,frequency,time,retention,location} })
}

#[tauri::command]
pub fn list_backups(db: State<Database>, token: String) -> Result<Vec<BackupRecord>, String> {
    let conn = db.conn.lock().unwrap(); authorized(&conn, &token, false)?;
    let mut stmt = conn.prepare("SELECT id,file_path,status FROM backups ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for item in rows.flatten() {
        if item.2 != "completed" { continue; }
        let path = std::path::Path::new(&item.1);
        let exists = path.exists() || path.file_name().map(|name| {
            let location = backup::backup_settings(&conn).4;
            std::path::Path::new(&location).join(name).exists()
        }).unwrap_or(false);
        if exists {
            if let Some(record) = row(&conn, item.0) { result.push(record); }
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn create_backup(db: State<Database>, token: String) -> BackupResponse {
    let conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse{success:false,message:e,backup:None}; }
    let location = backup::backup_settings(&conn).4;
    match backup::create_backup(&conn, std::path::Path::new(&location), "manual") {
        Ok((_,_,_)) => { let backup_id=conn.query_row("SELECT id FROM backups ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,i64>(0)).ok(); if let Some(id)=backup_id { audit::log(&conn,"backup_created",Some("backups"),Some(id),Some("Encrypted backup created and verified")); } BackupResponse{success:true,message:"Encrypted backup created and verified.".into(),backup:conn.query_row("SELECT id FROM backups ORDER BY id DESC LIMIT 1",[],|r|r.get::<_,i64>(0)).ok().and_then(|id|row(&conn,id))}},
        Err(e) => { audit::log(&conn,"backup_failed",Some("backups"),None,Some(&e)); BackupResponse{success:false,message:e,backup:None} }
    }
}

#[tauri::command]
pub fn verify_backup(db: State<Database>, token: String, backup_id: i64) -> BackupResponse {
    let conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse{success:false,message:e,backup:None}; }
    match backup::verify_backup(&conn, backup_id) {
        Ok(_) => { audit::log(&conn, "backup_verified", Some("backups"), Some(backup_id), Some("Backup integrity verified")); BackupResponse{success:true,message:"Backup verified successfully.".into(),backup:row(&conn,backup_id)} },
        Err(e)=> { audit::log(&conn, "backup_verification_failed", Some("backups"), Some(backup_id), Some(&e)); BackupResponse{success:false,message:e,backup:row(&conn,backup_id)} }
    }
}

#[tauri::command]
pub fn restore_backup_file(db: State<Database>, token: String, path: String) -> BackupResponse {
    let mut conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse{success:false,message:e,backup:None}; }
    match backup::restore_backup_file(&mut conn, std::path::Path::new(&path)) {
        Ok(_) => { audit::log(&conn, "backup_restored_from_file", Some("backups"), None, Some("Backup file restored successfully")); BackupResponse{success:true,message:"Backup restored and database upgraded to the current version. Please sign in again to continue.".into(),backup:None} },
        Err(e)=> { audit::log(&conn, "backup_restore_failed", Some("backups"), None, Some(&e)); BackupResponse{success:false,message:e,backup:None} }
    }
}

#[tauri::command]
pub fn restore_backup(db: State<Database>, token: String, backup_id: i64) -> BackupResponse {
    let mut conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse{success:false,message:e,backup:None}; }
    match backup::restore_backup(&mut conn, backup_id) {
        Ok(_) => { audit::log(&conn, "backup_restored", Some("backups"), Some(backup_id), Some("Backup restored successfully")); BackupResponse{success:true,message:"Backup restored successfully. Restart the application to reload all restored data.".into(),backup:row(&conn,backup_id)} },
        Err(e)=> { audit::log(&conn, "backup_restore_failed", Some("backups"), Some(backup_id), Some(&e)); BackupResponse{success:false,message:e,backup:row(&conn,backup_id)} }
    }
}

#[tauri::command]
pub fn delete_backup(db: State<Database>, token: String, backup_id: i64) -> BackupResponse {
    let conn = db.conn.lock().unwrap();
    if let Err(e) = authorized(&conn, &token, false) { return BackupResponse{success:false,message:e,backup:None}; }
    match backup::delete_backup(&conn, backup_id) {
        Ok(_) => { audit::log(&conn, "backup_deleted", Some("backups"), Some(backup_id), Some("Backup file and history deleted")); BackupResponse{success:true,message:"Backup deleted.".into(),backup:None} },
        Err(e)=> { audit::log(&conn, "backup_delete_failed", Some("backups"), Some(backup_id), Some(&e)); BackupResponse{success:false,message:e,backup:row(&conn,backup_id)} }
    }
}
