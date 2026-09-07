use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::{backup::Backup, Connection};
use std::{fs, path::PathBuf};
use crate::database::connection::Database;
use crate::security::session;
use crate::security::audit;
use crate::core::backup::{backup_settings, create_backup, default_backup_dir, restore_backup_file};
use crate::database::migrations;

#[derive(Serialize, Clone)]
pub struct GeneralSettings {
    pub app_name: String,
    pub language: String,
    pub currency: String,
    pub date_format: String,
    pub number_format: String,
    pub payroll_period_behavior: String,
}

#[derive(Deserialize)]
pub struct GeneralSettingsRequest {
    pub app_name: String,
    pub language: String,
    pub currency: String,
    pub date_format: String,
    pub number_format: String,
    pub payroll_period_behavior: String,
}

#[derive(Serialize, Clone)]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub channel: String,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub auto_check: bool,
    pub channel: String,
}


#[derive(Serialize, Clone)]
pub struct ReminderSettings {
    pub notifications_enabled: bool,
    pub polling_interval_seconds: i64,
    pub default_snooze_minutes: i64,
    pub default_category: String,
    pub show_completed: bool,
    pub auto_advance_recurring: bool,
}

#[derive(Deserialize)]
pub struct ReminderSettingsRequest {
    pub notifications_enabled: bool,
    pub polling_interval_seconds: i64,
    pub default_snooze_minutes: i64,
    pub default_category: String,
    pub show_completed: bool,
    pub auto_advance_recurring: bool,
}

#[derive(Serialize)]
pub struct SettingsResponse {
    pub success: bool,
    pub message: String,
}

fn require_admin(conn: &rusqlite::Connection, token: &str) -> Result<i64, String> {
    if token.trim().is_empty() { return Err("Authentication required".into()); }
    let mut stmt = conn.prepare("SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC")
        .map_err(|e| format!("Unable to validate session: {e}"))?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?)))
        .map_err(|e| format!("Unable to validate session: {e}"))?;
    for row in rows.flatten() {
        if let Ok(stored) = session::decrypt_token(&row.1) {
            if stored == token {
                if row.2 == 1 { return Err("Session is locked".into()); }
                return Ok(row.0);
            }
        }
    }
    Err("Session expired or not found".into())
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get::<_, String>(0)).ok()
}

fn set_setting(conn: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![key, value],
    ).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_general_settings(db: State<Database>) -> GeneralSettings {
    let conn = db.conn.lock().unwrap();
    GeneralSettings {
        app_name: get_setting(&conn, "app_name").unwrap_or_else(|| "Payroll System".into()),
        language: get_setting(&conn, "language").unwrap_or_else(|| "en".into()),
        currency: get_setting(&conn, "currency").unwrap_or_else(|| "RWF".into()),
        date_format: get_setting(&conn, "date_format").unwrap_or_else(|| "DD/MM/YYYY".into()),
        number_format: get_setting(&conn, "number_format").unwrap_or_else(|| "1,234.56".into()),
        payroll_period_behavior: get_setting(&conn, "payroll_period_behavior").unwrap_or_else(|| "monthly".into()),
    }
}

#[tauri::command]
pub fn update_general_settings(db: State<Database>, token: String, settings: GeneralSettingsRequest) -> SettingsResponse {
    let conn = db.conn.lock().unwrap();
    let admin_id = match require_admin(&conn, &token) {
        Ok(id) => id,
        Err(e) => return SettingsResponse { success: false, message: e },
    };

    let app_name = settings.app_name.trim();
    if app_name.is_empty() || app_name.len() > 100 {
        return SettingsResponse { success: false, message: "Application name must be 1-100 characters".into() };
    }
    let valid_languages = ["en", "fr", "rw"];
    if !valid_languages.contains(&settings.language.as_str()) {
        return SettingsResponse { success: false, message: "Invalid language selection".into() };
    }
    if settings.currency.trim().is_empty() || settings.currency.len() > 10 {
        return SettingsResponse { success: false, message: "Currency code must be 1-10 characters".into() };
    }
    let valid_date_formats = ["DD/MM/YYYY", "MM/DD/YYYY", "YYYY-MM-DD"];
    if !valid_date_formats.contains(&settings.date_format.as_str()) {
        return SettingsResponse { success: false, message: "Invalid date format".into() };
    }
    let valid_number_formats = ["1,234.56", "1.234,56", "1 234.56"];
    if !valid_number_formats.contains(&settings.number_format.as_str()) {
        return SettingsResponse { success: false, message: "Invalid number format".into() };
    }
    let valid_behaviors = ["monthly", "bi-weekly", "weekly"];
    if !valid_behaviors.contains(&settings.payroll_period_behavior.as_str()) {
        return SettingsResponse { success: false, message: "Invalid payroll period behavior".into() };
    }

    let pairs = [
        ("app_name", app_name),
        ("language", &settings.language),
        ("currency", settings.currency.trim()),
        ("date_format", &settings.date_format),
        ("number_format", &settings.number_format),
        ("payroll_period_behavior", &settings.payroll_period_behavior),
    ];
    for (key, value) in &pairs {
        if let Err(e) = set_setting(&conn, key, value) {
            return SettingsResponse { success: false, message: format!("Unable to save {key}: {e}") };
        }
    }

    audit::log(&conn, "general_settings_changed", Some("app_settings"), Some(admin_id), Some(&format!("app={}, lang={}, currency={}", app_name, settings.language, settings.currency)));

    SettingsResponse { success: true, message: "General settings saved".into() }
}

#[tauri::command]
pub fn get_update_settings(db: State<Database>) -> UpdateSettings {
    let conn = db.conn.lock().unwrap();
    UpdateSettings {
        auto_check: get_setting(&conn, "update_auto_check").map(|v| v == "1").unwrap_or(true),
        channel: get_setting(&conn, "update_channel").unwrap_or_else(|| "stable".into()),
    }
}

#[tauri::command]
pub fn update_update_settings(db: State<Database>, token: String, settings: UpdateSettingsRequest) -> SettingsResponse {
    let conn = db.conn.lock().unwrap();
    let admin_id = match require_admin(&conn, &token) {
        Ok(id) => id,
        Err(e) => return SettingsResponse { success: false, message: e },
    };

    let valid_channels = ["stable", "beta"];
    if !valid_channels.contains(&settings.channel.as_str()) {
        return SettingsResponse { success: false, message: "Invalid update channel".into() };
    }

    if let Err(e) = set_setting(&conn, "update_auto_check", if settings.auto_check { "1" } else { "0" }) {
        return SettingsResponse { success: false, message: format!("Unable to save: {e}") };
    }
    if let Err(e) = set_setting(&conn, "update_channel", &settings.channel) {
        return SettingsResponse { success: false, message: format!("Unable to save: {e}") };
    }

    audit::log(&conn, "update_settings_changed", Some("app_settings"), Some(admin_id), Some(&format!("auto_check={}, channel={}", settings.auto_check, settings.channel)));

    SettingsResponse { success: true, message: "Update settings saved".into() }
}



#[tauri::command]
pub fn get_reminder_settings(db: State<Database>) -> ReminderSettings {
    let conn = db.conn.lock().unwrap();
    ReminderSettings {
        notifications_enabled: get_setting(&conn, "reminder_notifications_enabled").map(|v| v == "1").unwrap_or(true),
        polling_interval_seconds: get_setting(&conn, "reminder_polling_interval_seconds").and_then(|v| v.parse().ok()).filter(|v: &i64| (5..=3600).contains(v)).unwrap_or(15),
        default_snooze_minutes: get_setting(&conn, "reminder_default_snooze_minutes").and_then(|v| v.parse().ok()).filter(|v: &i64| (1..=10080).contains(v)).unwrap_or(60),
        default_category: get_setting(&conn, "reminder_default_category").filter(|v| ["general","payroll","leave","loan","backup","update"].contains(&v.as_str())).unwrap_or_else(|| "general".into()),
        show_completed: get_setting(&conn, "reminder_show_completed").map(|v| v == "1").unwrap_or(true),
        auto_advance_recurring: get_setting(&conn, "reminder_auto_advance_recurring").map(|v| v == "1").unwrap_or(true),
    }
}

#[tauri::command]
pub fn update_reminder_settings(db: State<Database>, token: String, settings: ReminderSettingsRequest) -> SettingsResponse {
    let conn = db.conn.lock().unwrap();
    let admin_id = match require_admin(&conn, &token) {
        Ok(id) => id,
        Err(e) => return SettingsResponse { success: false, message: e },
    };
    if !(5..=3600).contains(&settings.polling_interval_seconds) {
        return SettingsResponse { success: false, message: "Polling interval must be between 5 and 3600 seconds.".into() };
    }
    if !(1..=10080).contains(&settings.default_snooze_minutes) {
        return SettingsResponse { success: false, message: "Default snooze must be between 1 minute and 7 days.".into() };
    }
    if !["general", "payroll", "leave", "loan", "backup", "update"].contains(&settings.default_category.as_str()) {
        return SettingsResponse { success: false, message: "Invalid default reminder category.".into() };
    }
    let values = [
        ("reminder_notifications_enabled", if settings.notifications_enabled { "1" } else { "0" }.to_string()),
        ("reminder_polling_interval_seconds", settings.polling_interval_seconds.to_string()),
        ("reminder_default_snooze_minutes", settings.default_snooze_minutes.to_string()),
        ("reminder_default_category", settings.default_category.clone()),
        ("reminder_show_completed", if settings.show_completed { "1" } else { "0" }.to_string()),
        ("reminder_auto_advance_recurring", if settings.auto_advance_recurring { "1" } else { "0" }.to_string()),
    ];
    for (key, value) in &values {
        if let Err(e) = set_setting(&conn, key, value) {
            return SettingsResponse { success: false, message: format!("Unable to save {key}: {e}") };
        }
    }
    audit::log(&conn, "reminder_settings_changed", Some("app_settings"), Some(admin_id), Some(&format!(
        "enabled={}, polling={}s, snooze={}m, category={}, show_completed={}, auto_advance={}",
        settings.notifications_enabled, settings.polling_interval_seconds, settings.default_snooze_minutes,
        settings.default_category, settings.show_completed, settings.auto_advance_recurring
    )));
    SettingsResponse { success: true, message: "Reminder settings saved".into() }
}


#[derive(Serialize)]
pub struct ResetResponse {
    pub success: bool,
    pub message: String,
}

fn reset_database(conn: &mut Connection, admin_id: i64) -> Result<(), String> {
    let backup_location = backup_settings(conn).4;
    let (backup_path, backup_size, backup_checksum) = create_backup(conn, std::path::Path::new(&backup_location), "pre_reset")?;
    let backup_id: i64 = conn.query_row("SELECT id FROM backups WHERE file_path = ?1 ORDER BY id DESC LIMIT 1", [&backup_path], |r| r.get(0))
        .map_err(|e| format!("Unable to record the reset safety backup: {e}"))?;

    let admin: (i64, String, String, Option<String>, String, String) = conn.query_row(
        "SELECT id, username, password_hash, recovery_hash, created_at, updated_at FROM admin_users WHERE id = ?1",
        [admin_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    ).map_err(|e| format!("Unable to preserve the administrator account: {e}"))?;

    let fresh_path: PathBuf = default_backup_dir().join(format!("settings_reset_{}.sqlite", std::process::id()));
    let _ = fs::remove_file(&fresh_path);
    let result = (|| -> Result<(), String> {
        let fresh = Connection::open(&fresh_path).map_err(|e| format!("Unable to prepare clean database: {e}"))?;
        fresh.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;
        migrations::run(&fresh).map_err(|e| format!("Unable to initialize clean database: {e}"))?;

        fresh.execute(
            "INSERT INTO admin_users (id, username, password_hash, recovery_hash, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
            rusqlite::params![admin.0, admin.1, admin.2, admin.3, admin.4, admin.5],
        ).map_err(|e| format!("Unable to preserve the administrator account: {e}"))?;

        let database_version: String = conn.query_row("SELECT COALESCE(MAX(version),0) FROM schema_migrations", [], |r| r.get::<_, i64>(0)).unwrap_or(0).to_string();
        let app_version: String = conn.query_row("SELECT value FROM app_settings WHERE key='app_version'", [], |r| r.get(0)).unwrap_or_else(|_| "1.0.0".into());
        fresh.execute(
            "INSERT INTO backups (id,file_path,file_size,backup_type,status,checksum,encrypted,database_version,app_version) VALUES (?1,?2,?3,'pre_reset','completed',?4,1,?5,?6)",
            rusqlite::params![backup_id, backup_path, backup_size as i64, backup_checksum, database_version, app_version],
        ).map_err(|e| format!("Unable to preserve the reset safety backup: {e}"))?;

        fresh.execute(
            "INSERT INTO audit_logs (action, entity_type, entity_id, details) VALUES ('system_reset_completed','system',?1,'Full system reset completed; administrator account and required system defaults preserved.')",
            [admin_id],
        ).map_err(|e| format!("Unable to record reset audit event: {e}"))?;
        fresh.execute_batch("PRAGMA integrity_check;").map_err(|e| format!("Clean database verification failed: {e}"))?;
        let check: String = fresh.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        if check != "ok" { return Err(format!("Clean database integrity check failed: {check}")); }
        drop(fresh);

        conn.execute_batch("PRAGMA foreign_keys=OFF;").map_err(|e| e.to_string())?;
        let source = Connection::open(&fresh_path).map_err(|e| format!("Unable to reopen clean database: {e}"))?;
        {
            let copy = Backup::new(&source, conn).map_err(|e| format!("Unable to replace application database: {e}"))?;
            copy.run_to_completion(5, std::time::Duration::from_millis(20), None).map_err(|e| format!("Unable to replace application database: {e}"))?;
        }
        drop(source);
        conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| format!("Unable to re-enable database constraints: {e}"))?;
        migrations::run(conn).map_err(|e| format!("Database verification after reset failed: {e}"))?;
        let admin_count: i64 = conn.query_row("SELECT COUNT(*) FROM admin_users", [], |r| r.get(0)).unwrap_or(0);
        let settings_count: i64 = conn.query_row("SELECT COUNT(*) FROM app_settings", [], |r| r.get(0)).unwrap_or(0);
        if admin_count != 1 || settings_count < 1 { return Err("Reset verification failed: required system records are missing.".into()); }
        Ok(())
    })();

    let _ = fs::remove_file(&fresh_path);
    if result.is_ok() {
        Ok(())
    } else {
        // Re-use the existing, verified backup/restore engine for rollback.
        conn.execute_batch("PRAGMA foreign_keys=OFF;").ok();
        let rollback = restore_backup_file(conn, std::path::Path::new(&backup_path));
        conn.execute_batch("PRAGMA foreign_keys=ON;").ok();
        match rollback {
            Ok(()) => Err(format!("System reset failed; the original database was restored safely. {}", result.err().unwrap_or_default())),
            Err(e) => Err(format!("System reset failed and automatic rollback failed: {}. Recovery backup: {}", result.err().unwrap_or_default(), e)),
        }
    }
}

#[tauri::command]
pub fn reset_system(db: State<Database>, token: String) -> ResetResponse {
    let mut conn = db.conn.lock().unwrap();
    let admin_id = match require_admin(&conn, &token) {
        Ok(id) => id,
        Err(e) => return ResetResponse { success: false, message: e },
    };
    match reset_database(&mut conn, admin_id) {
        Ok(()) => ResetResponse { success: true, message: "System reset completed successfully. The application is ready for a fresh sign-in.".into() },
        Err(e) => ResetResponse { success: false, message: e },
    }
}
