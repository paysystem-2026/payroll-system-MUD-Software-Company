use serde::Serialize;
use sha2::{Digest, Sha256};
use semver::Version;
use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

use crate::core::backup;
use crate::database::connection::Database;
use crate::security::{audit, session};

const CHECK_INTERVAL_SECS: i64 = 6 * 60 * 60;
const UPDATE_ENDPOINT: Option<&str> = option_env!("PAYROLL_UPDATE_ENDPOINT");
const UPDATE_PUBKEY: Option<&str> = option_env!("PAYROLL_UPDATE_PUBKEY");

#[derive(Clone, Serialize, Default)]
pub struct UpdateMetadata {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
    pub size_bytes: Option<u64>,
    pub target: String,
    pub platform: String,
    pub architecture: String,
    pub compatible: bool,
    pub signature_present: bool,
}

#[derive(Clone, Serialize, Default)]
pub struct UpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f64>,
}

#[derive(Clone, Serialize, Default)]
pub struct UpdateStatus {
    pub current_version: String,
    pub state: String,
    pub message: String,
    pub update: Option<UpdateMetadata>,
    pub progress: UpdateProgress,
    pub last_checked: Option<String>,
    pub service_configured: bool,
    pub source: String,
    pub platform: String,
    pub architecture: String,
    pub checksum_present: bool,
    pub recovery_available: bool,
    pub recovery_created_at: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct UpdateHistoryEntry {
    pub id: i64,
    pub version_from: Option<String>,
    pub version_to: String,
    pub status: String,
    pub event_type: Option<String>,
    pub details: Option<String>,
    pub size_bytes: Option<u64>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct UpdateStatusResponse {
    pub status: UpdateStatus,
    pub history: Vec<UpdateHistoryEntry>,
}

#[derive(Serialize)]
pub struct UpdateActionResponse {
    pub success: bool,
    pub message: String,
}

pub struct UpdateState {
    pub status: Mutex<UpdateStatus>,
    pub pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    pub downloaded_package: Mutex<Option<Vec<u8>>>,
    pub cancel_download: AtomicBool,
    pub download_in_progress: AtomicBool,
    pub check_in_progress: AtomicBool,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus {
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                state: "idle".into(),
                message: "Ready to check for updates.".into(),
                update: None,
                progress: UpdateProgress::default(),
                last_checked: None,
                service_configured: configured(),
                source: update_source_label(),
                platform: std::env::consts::OS.into(),
                architecture: std::env::consts::ARCH.into(),
                checksum_present: false,
                recovery_available: false,
                recovery_created_at: None,
            }),
            pending_update: Mutex::new(None),
            downloaded_package: Mutex::new(None),
            cancel_download: AtomicBool::new(false),
            download_in_progress: AtomicBool::new(false),
            check_in_progress: AtomicBool::new(false),
        }
    }
}

fn update_source_label() -> String {
    match UPDATE_ENDPOINT.map(str::trim) {
        Some(endpoint) if endpoint.contains("github.com/") => "GitHub Releases".into(),
        Some(endpoint) if !endpoint.is_empty() => "Configured update server".into(),
        _ => "Not configured".into(),
    }
}

fn configured() -> bool {
    UPDATE_ENDPOINT.map(str::trim).filter(|v| !v.is_empty()).is_some()
        && UPDATE_PUBKEY.map(str::trim).filter(|v| !v.is_empty()).is_some()
}

fn endpoint(channel: &str) -> Result<url::Url, String> {
    let raw = UPDATE_ENDPOINT.ok_or_else(|| "Update service is not configured for this build.".to_string())?;
    let raw = raw.trim();
    if raw.is_empty() { return Err("Update service is not configured for this build.".into()); }
    let replaced = raw.replace("{channel}", channel);
    replaced.parse::<url::Url>().map_err(|e| format!("Invalid update endpoint configuration: {e}"))
}

fn updater(app: &AppHandle, channel: &str) -> Result<tauri_plugin_updater::Updater, String> {
    let pubkey = UPDATE_PUBKEY.ok_or_else(|| "Update signing key is not configured for this build.".to_string())?;
    if pubkey.trim().is_empty() { return Err("Update signing key is not configured for this build.".into()); }
    app.updater_builder()
        .endpoints(vec![endpoint(channel)?])
        .map_err(|e| format!("Unable to configure updater: {e}"))?
        .pubkey(pubkey)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Unable to initialize updater: {e}"))
}

fn authorized(conn: &rusqlite::Connection, token: &str) -> Result<i64, String> {
    if token.trim().is_empty() { return Err("Authentication required".into()); }
    let mut stmt = conn.prepare("SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC")
        .map_err(|e| format!("Unable to validate session: {e}"))?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))
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

fn setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get(0)).ok()
}

fn now(conn: &rusqlite::Connection) -> Option<String> {
    conn.query_row("SELECT datetime('now')", [], |r| r.get(0)).ok()
}

fn record(conn: &rusqlite::Connection, event_type: &str, version_from: &str, version_to: &str, status: &str, details: &str, size: Option<u64>) {
    let _ = conn.execute(
        "INSERT INTO update_history(version_from, version_to, status, event_type, details, size_bytes) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![version_from, version_to, status, event_type, details, size.map(|v| v as i64)],
    );
}

fn history(conn: &rusqlite::Connection) -> Vec<UpdateHistoryEntry> {
    let mut stmt = match conn.prepare("SELECT id,version_from,version_to,status,event_type,details,size_bytes,created_at FROM update_history ORDER BY id DESC LIMIT 50") {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([], |r| Ok(UpdateHistoryEntry {
        id: r.get(0)?, version_from: r.get(1)?, version_to: r.get(2)?, status: r.get(3)?, event_type: r.get(4)?, details: r.get(5)?, size_bytes: r.get::<_, Option<i64>>(6)?.map(|v| v as u64), created_at: r.get(7)?,
    })) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    rows.flatten().collect()
}

fn emit_status(app: &AppHandle, state: &UpdateState) {
    if let Ok(status) = state.status.lock() {
        let _ = app.emit("update://status", status.clone());
    }
}

fn metadata(update: &tauri_plugin_updater::Update) -> UpdateMetadata {
    let size = update.raw_json.get("size").and_then(|v| v.as_u64()).or_else(|| {
        update.raw_json.get("platforms")
            .and_then(|platforms| platforms.get(&update.target))
            .and_then(|target| target.get("size"))
            .and_then(|v| v.as_u64())
    });
    let compatible = tauri_plugin_updater::target().map(|expected| update.target == expected).unwrap_or(false);
    UpdateMetadata {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
        date: update.date.map(|d| d.to_string()),
        size_bytes: size,
        target: update.target.clone(),
        platform: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        compatible,
        signature_present: !update.signature.trim().is_empty(),
    }
}

fn expected_checksum(update: &tauri_plugin_updater::Update) -> Option<String> {
    let candidate = update.raw_json.get("sha256")
        .or_else(|| update.raw_json.get("checksum"))
        .or_else(|| update.raw_json.get("platforms")
            .and_then(|platforms| platforms.get(&update.target))
            .and_then(|target| target.get("sha256").or_else(|| target.get("checksum"))));
    candidate
        .and_then(|v| v.as_str())
        .map(|v| v.trim().trim_start_matches("sha256:").to_ascii_lowercase())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn meta_checksum_present(update: &tauri_plugin_updater::Update) -> bool { expected_checksum(update).is_some() }

fn checksum_matches(update: &tauri_plugin_updater::Update, bytes: &[u8]) -> Result<String, String> {
    let actual = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
    if let Some(expected) = expected_checksum(update) {
        if expected != actual {
            return Err("Downloaded update failed SHA-256 checksum validation.".into());
        }
        return Ok(actual);
    }
    Ok(actual)
}

fn recovery_info(conn: &rusqlite::Connection) -> (bool, Option<String>) {
    conn.query_row(
        "SELECT created_at FROM backups WHERE backup_type = 'pre_update' AND status = 'completed' ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get::<_, String>(0),
    ).map(|created| (true, Some(created))).unwrap_or((false, None))
}

fn set_recovery_status(conn: &rusqlite::Connection, status: &mut UpdateStatus) {
    let (available, created_at) = recovery_info(conn);
    status.recovery_available = available;
    status.recovery_created_at = created_at;
}


struct CheckGuard<'a>(&'a AtomicBool);
impl Drop for CheckGuard<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }

struct DownloadGuard<'a>(&'a AtomicBool);
impl Drop for DownloadGuard<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }

async fn check_internal(app: AppHandle, force: bool) -> Result<Option<UpdateMetadata>, String> {
    let state = app.state::<UpdateState>();
    let _check_guard = match state.check_in_progress.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => CheckGuard(&state.check_in_progress),
        Err(_) => return Err("An update check is already in progress.".into()),
    };
    let db = app.state::<Database>();
    let channel = {
        let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
        let auto = setting(&conn, "update_auto_check").map(|v| v != "0").unwrap_or(true);
        if !force && !auto { return Ok(None); }
        if !force {
            if let Some(last) = setting(&conn, "update_last_check_epoch").and_then(|v| v.parse::<i64>().ok()) {
                let current = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                if current - last < CHECK_INTERVAL_SECS { return Ok(None); }
            }
        }
        setting(&conn, "update_channel").unwrap_or_else(|| "stable".into())
    };

    if !configured() {
        let state = app.state::<UpdateState>();
        let db = app.state::<Database>();
        let checked_at = if let Ok(conn) = db.conn.lock() {
            record(&conn, "check", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_VERSION"), "unavailable", "Update service is not configured for this build.", None);
            audit::log(&conn, "update_check_unavailable", Some("update_history"), None, Some("Updater endpoint or signing key is not configured"));
            now(&conn)
        } else { None };
        if let Ok(mut status) = state.status.lock() {
            status.state = "unavailable".into();
            status.message = "Update service is unavailable for this build. The application will continue normally.".into();
            status.last_checked = checked_at;
            status.checksum_present = false;
        }
        emit_status(&app, &state);
        return Err("Update service is unavailable for this build. The application will continue normally.".into());
    }

    if let Ok(conn) = db.conn.lock() { audit::log(&conn, "update_check_started", Some("update_history"), None, Some(&format!("channel={channel}"))); }
    if let Ok(mut s) = state.status.lock() { s.state = "checking".into(); s.message = "Checking for updates…".into(); s.progress = UpdateProgress::default(); }
    emit_status(&app, &state);
    let updater = match updater(&app, &channel) {
        Ok(value) => value,
        Err(e) => {
            if let Ok(conn) = db.conn.lock() {
                record(&conn, "check", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_VERSION"), "failed", &e, None);
                audit::log(&conn, "update_check_failed", Some("update_history"), None, Some(&e));
            }
            if let Ok(mut s) = state.status.lock() { s.state = "error".into(); s.message = e.clone(); }
            emit_status(&app, &state);
            return Err(e);
        }
    };
    let result = updater.check().await.map_err(|e| format!("Update check failed: {e}"));
    let db = app.state::<Database>();
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let current = env!("CARGO_PKG_VERSION");
    let epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_else(|_| "0".into());
    let _ = conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('update_last_check_epoch',?1,datetime('now'))", [&epoch]);
    let checked_at = now(&conn);

    match result {
        Ok(Some(update)) => {
            let meta = metadata(&update);
            let current_version = Version::parse(current).map_err(|e| format!("Current application version is invalid: {e}"))?;
            let update_version = Version::parse(meta.version.trim_start_matches('v')).map_err(|e| format!("Update version is invalid: {e}"))?;
            if update_version <= current_version {
                record(&conn, "validation", current, &meta.version, "validation_failed", "Update version is not newer than the installed version.", meta.size_bytes);
                drop(conn);
                if let Ok(mut s) = state.status.lock() { s.state="rejected".into(); s.message="Update rejected: version ordering is invalid.".into(); s.update=Some(meta.clone()); s.checksum_present=meta_checksum_present(&update); s.last_checked=checked_at; }
                emit_status(&app, &state);
                return Err("Update rejected: version ordering is invalid.".into());
            }
            if !meta.compatible || !meta.signature_present {
                record(&conn, "validation", current, &meta.version, "validation_failed", "Update target or signature metadata is invalid.", meta.size_bytes);
                drop(conn);
                if let Ok(mut s) = state.status.lock() { s.state="rejected".into(); s.message="Update rejected: incompatible target or missing signature.".into(); s.update=Some(meta.clone()); s.checksum_present=meta_checksum_present(&update); s.last_checked=checked_at; }
                emit_status(&app, &state);
                return Err("Update rejected: incompatible target or missing signature.".into());
            }
            let checksum_present = meta_checksum_present(&update);
            record(&conn, "available", current, &meta.version, "available", "A signed update is available.", meta.size_bytes);
            audit::log(&conn, "update_available", Some("update_history"), None, Some(&format!("Signed update {} is available", meta.version)));
            drop(conn);
            if let Ok(mut pending) = state.pending_update.lock() { *pending = Some(update); }
            if let Ok(mut bytes) = state.downloaded_package.lock() { *bytes = None; }
            if let Ok(mut s) = state.status.lock() { s.state="available".into(); s.message="Update available".into(); s.update=Some(meta.clone()); s.checksum_present=checksum_present; s.progress=UpdateProgress::default(); s.last_checked=checked_at; }
            emit_status(&app, &state);
            Ok(Some(meta))
        }
        Ok(None) => {
            record(&conn, "check", current, current, "checked", "No update is available.", None);
            audit::log(&conn, "update_check_completed", Some("update_history"), None, Some("No update available"));
            drop(conn);
            if let Ok(mut s) = state.status.lock() { s.state="up_to_date".into(); s.message="You're up to date".into(); s.update=None; s.checksum_present=false; s.progress=UpdateProgress::default(); s.last_checked=checked_at; }
            emit_status(&app, &state);
            Ok(None)
        }
        Err(e) => {
            record(&conn, "check", current, current, "failed", &e, None);
            audit::log(&conn, "update_check_failed", Some("update_history"), None, Some(&e));
            drop(conn);
            if let Ok(mut s) = state.status.lock() { s.state="error".into(); s.message=e.clone(); s.checksum_present=false; s.last_checked=checked_at; }
            emit_status(&app, &state);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> UpdateActionResponse {
    match check_internal(app, true).await {
        Ok(Some(meta)) => UpdateActionResponse { success:true, message: format!("Update available: {}", meta.version) },
        Ok(None) => UpdateActionResponse { success:true, message:"You're up to date".into() },
        Err(e) => UpdateActionResponse { success:false, message:e },
    }
}

#[tauri::command]
pub fn get_update_status(db: State<Database>, state: State<UpdateState>) -> Result<UpdateStatusResponse, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let mut status = state.status.lock().map_err(|_| "Update state unavailable".to_string())?.clone();
    status.service_configured = configured();
    status.source = update_source_label();
    set_recovery_status(&conn, &mut status);
    Ok(UpdateStatusResponse { status, history: history(&conn) })
}

#[tauri::command]
pub async fn download_update(app: AppHandle, state: State<'_, UpdateState>) -> Result<UpdateActionResponse, String> {
    let _download_guard = match state.download_in_progress.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => DownloadGuard(&state.download_in_progress),
        Err(_) => return Ok(UpdateActionResponse { success:false, message:"An update download is already in progress.".into() }),
    };
    let update = match state.pending_update.lock().ok().and_then(|mut g| g.take()) {
        Some(update) => update,
        None => return Ok(UpdateActionResponse { success:false, message:"No validated update is pending. Check for updates again.".into() }),
    };
    state.cancel_download.store(false, Ordering::SeqCst);
    let current = env!("CARGO_PKG_VERSION").to_string();
    let meta = metadata(&update);
    if !meta.compatible || !meta.signature_present {
        if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
        return Ok(UpdateActionResponse { success:false, message:"Update validation failed before download.".into() });
    }
    {
        let db = app.state::<Database>();
        if let Ok(conn) = db.conn.lock() { record(&conn, "download", &current, &meta.version, "download_started", "Update download started.", meta.size_bytes); audit::log(&conn, "update_download_started", Some("update_history"), None, Some(&meta.version)); };
    }
    if let Ok(mut s) = state.status.lock() { s.state="downloading".into(); s.message="Downloading update…".into(); s.progress=UpdateProgress::default(); }
    emit_status(&app, &state);

    let app_for_progress = app.clone();
    let state_ptr = &*state;
    let mut downloaded = 0u64;
    let result = update.download(
        move |chunk, total| {
            downloaded = downloaded.saturating_add(chunk as u64);
            if let Ok(mut s) = state_ptr.status.lock() {
                s.progress = UpdateProgress { downloaded_bytes: downloaded, total_bytes: total, percent: total.map(|t| if t == 0 { 0.0 } else { downloaded as f64 * 100.0 / t as f64 }) };
            }
            emit_status(&app_for_progress, state_ptr);
        },
        || {},
    ).await;

    match result {
        Ok(bytes) => {
            if state.cancel_download.load(Ordering::SeqCst) {
                let db = app.state::<Database>();
                if let Ok(conn) = db.conn.lock() { record(&conn, "download", &current, &meta.version, "cancelled", "Update download cancelled by user.", meta.size_bytes); audit::log(&conn, "update_download_cancelled", Some("update_history"), None, Some(&meta.version)); };
                if let Ok(mut s) = state.status.lock() { s.state="available".into(); s.message="Download cancelled. The current application is unchanged.".into(); }
                if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
                emit_status(&app, &state);
                return Ok(UpdateActionResponse { success:false, message:"Download cancelled.".into() });
            }
            let verified_checksum = match checksum_matches(&update, &bytes) {
                Ok(value) => value,
                Err(e) => {
                    let db = app.state::<Database>();
                    if let Ok(conn) = db.conn.lock() { record(&conn, "validation", &current, &meta.version, "validation_failed", &e, meta.size_bytes); audit::log(&conn, "update_verification_failed", Some("update_history"), None, Some(&e)); };
                    if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
                    if let Ok(mut s) = state.status.lock() { s.state="rejected".into(); s.message=e.clone(); }
                    emit_status(&app, &state);
                    return Ok(UpdateActionResponse { success:false, message:e });
                }
            };
            let checksum_present = meta_checksum_present(&update);
            let downloaded_size = bytes.len();
            if let Ok(mut p) = state.downloaded_package.lock() { *p = Some(bytes); }
            if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
            let db = app.state::<Database>();
            if let Ok(conn) = db.conn.lock() { record(&conn, "download", &current, &meta.version, "download_completed", "Update downloaded and cryptographically verified.", meta.size_bytes); audit::log(&conn, "update_verification_completed", Some("update_history"), None, Some(&format!("{} bytes; sha256={}", downloaded_size, verified_checksum))); };
            if let Ok(mut s) = state.status.lock() { s.state="downloaded".into(); s.message="Update downloaded and verified. Ready to install.".into(); s.checksum_present=checksum_present; }
            emit_status(&app, &state);
            Ok(UpdateActionResponse { success:true, message:"Update downloaded and verified. Ready to install.".into() })
        }
        Err(e) => {
            let msg = format!("Update download failed: {e}");
            let db = app.state::<Database>();
            if let Ok(conn) = db.conn.lock() { record(&conn, "download", &current, &meta.version, "download_failed", &msg, meta.size_bytes); audit::log(&conn, "update_download_failed", Some("update_history"), None, Some(&msg)); };
            if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
            if let Ok(mut s) = state.status.lock() { s.state="error".into(); s.message=msg.clone(); }
            emit_status(&app, &state);
            Ok(UpdateActionResponse { success:false, message:msg })
        }
    }
}

#[tauri::command]
pub fn cancel_update_download(state: State<UpdateState>) -> UpdateActionResponse {
    state.cancel_download.store(true, Ordering::SeqCst);
    UpdateActionResponse { success:true, message:"Cancellation requested.".into() }
}

#[tauri::command]
pub fn install_update(app: AppHandle, db: State<Database>, state: State<UpdateState>, token: String) -> UpdateActionResponse {
    let conn = match db.conn.lock() { Ok(c) => c, Err(_) => return UpdateActionResponse{success:false,message:"Database lock unavailable".into()} };
    let admin_id = match authorized(&conn, &token) { Ok(id)=>id, Err(e)=>return UpdateActionResponse{success:false,message:e} };
    let update = match state.pending_update.lock().ok().and_then(|mut g| g.take()) { Some(v)=>v, None=>return UpdateActionResponse{success:false,message:"No validated update is pending. Check for updates again.".into()} };
    let bytes = match state.downloaded_package.lock().ok().and_then(|mut g| g.take()) {
        Some(v) => v,
        None => {
            if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
            return UpdateActionResponse{success:false,message:"Download and verify the update before installing.".into()};
        }
    };
    let current = env!("CARGO_PKG_VERSION");
    let meta = metadata(&update);
    if !meta.compatible || !meta.signature_present {
        if let Ok(mut p) = state.downloaded_package.lock() { *p = Some(bytes); }
        if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
        return UpdateActionResponse{success:false,message:"Update validation failed. Installation was blocked.".into()};
    }
    let location = backup::backup_settings(&conn).4;
    record(&conn, "install", current, &meta.version, "install_started", "Creating a protected database backup before installation.", meta.size_bytes);
    let backup_result = backup::create_backup(&conn, std::path::Path::new(&location), "pre_update");
    if let Err(e) = backup_result {
        record(&conn, "recovery", current, &meta.version, "install_failed", &format!("Pre-update backup failed: {e}"), meta.size_bytes);
        audit::log(&conn, "update_backup_failed", Some("update_history"), Some(admin_id), Some(&e));
        drop(conn);
        if let Ok(mut p) = state.downloaded_package.lock() { *p = Some(bytes); }
        if let Ok(mut p) = state.pending_update.lock() { *p = Some(update); }
        return UpdateActionResponse{success:false,message:format!("Installation blocked because the pre-update backup failed: {e}")};
    }
    record(&conn, "backup", current, &meta.version, "completed", "Protected pre-update database backup created.", meta.size_bytes);
    audit::log(&conn, "update_backup_completed", Some("update_history"), Some(admin_id), Some(&format!("Protected pre-update database backup created for {}", meta.version)));
    audit::log(&conn, "update_install_started", Some("update_history"), Some(admin_id), Some(&format!("{} -> {}", current, meta.version)));
    drop(conn);
    if let Ok(mut s) = state.status.lock() { s.state="installing".into(); s.message="Installing update…".into(); }
    emit_status(&app, &state);

    let retry_bytes = bytes.clone();
    let retry_update = update.clone();
    match update.install(bytes) {
        Ok(()) => {
            let db = app.state::<Database>();
            if let Ok(conn) = db.conn.lock() {
                record(&conn, "install", current, &meta.version, "installed", "Update installer accepted the verified package. Restart is required to run the new version.", meta.size_bytes);
                audit::log(&conn, "update_install_completed", Some("update_history"), Some(admin_id), Some(&format!("{} -> {}", current, meta.version)));
            }
            if let Ok(mut s) = state.status.lock() { s.state="installed_pending_restart".into(); s.message="Update installed. Restart the application to finish the update.".into(); }
            emit_status(&app, &state);
            UpdateActionResponse{success:true,message:"Update installed. Restart the application to finish the update.".into()}
        }
        Err(e) => {
            let msg = format!("Update installation failed: {e}");
            if let Ok(mut p) = state.downloaded_package.lock() { *p = Some(retry_bytes); }
            if let Ok(mut p) = state.pending_update.lock() { *p = Some(retry_update); }
            let db = app.state::<Database>();
            if let Ok(conn) = db.conn.lock() { record(&conn, "install", current, &meta.version, "install_failed", &msg, meta.size_bytes); audit::log(&conn, "update_install_failed", Some("update_history"), Some(admin_id), Some(&msg)); }
            if let Ok(mut s) = state.status.lock() { s.state="error".into(); s.message=msg.clone(); }
            emit_status(&app, &state);
            UpdateActionResponse{success:false,message:msg}
        }
    }
}

#[tauri::command]
pub fn restart_update(app: AppHandle) -> UpdateActionResponse {
    let db = app.state::<Database>();
    if let Ok(conn) = db.conn.lock() {
        audit::log(&conn, "update_restart_requested", Some("update_history"), None, Some("Application restart requested to complete update"));
    }
    app.restart();
}

pub fn start_background_checker(app: AppHandle) {
    std::thread::spawn(move || {
        // Keep all network work outside the UI/startup path. The six-hour guard in check_internal
        // prevents duplicate requests after manual checks or restarts.
        std::thread::sleep(Duration::from_secs(3));
        loop {
            let _ = tauri::async_runtime::block_on(check_internal(app.clone(), false));
            std::thread::sleep(Duration::from_secs(CHECK_INTERVAL_SECS as u64));
        }
    });
}
