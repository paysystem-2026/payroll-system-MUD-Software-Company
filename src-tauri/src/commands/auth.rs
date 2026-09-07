use rusqlite::OptionalExtension;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::connection::Database;
use crate::security::{audit, password, session};

#[derive(Serialize)]
pub struct AuthStatus {
    pub is_setup: bool,
    pub is_authenticated: bool,
    pub is_locked: bool,
    pub admin_username: Option<String>,
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct SetupRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub username: String,
    pub recovery_code: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
    pub recovery_code: Option<String>,
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .ok()
}

fn admin_exists(conn: &rusqlite::Connection) -> bool {
    conn.query_row("SELECT COUNT(*) FROM admin_users", [], |row| {
        row.get::<_, i64>(0)
    })
    .unwrap_or(0)
        > 0
}

fn find_admin_by_username(
    conn: &rusqlite::Connection,
    username: &str,
) -> Option<(i64, String, String)> {
    conn.query_row(
        "SELECT id, username, password_hash FROM admin_users WHERE username = ?1",
        [username],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .ok()
}

fn create_session(
    conn: &rusqlite::Connection,
    admin_id: i64,
    timeout_minutes: i64,
) -> Result<String, String> {
    let token = session::generate_session_token();
    let encrypted = session::encrypt_token(&token)?;

    conn.execute(
        "INSERT INTO admin_sessions (admin_id, session_token, expires_at, last_activity) VALUES (?1, ?2, datetime('now', '+' || ?3 || ' minutes'), datetime('now'))",
        rusqlite::params![admin_id, &encrypted, timeout_minutes],
    )
    .map_err(|e| format!("Session error: {}", e))?;

    Ok(token)
}

fn find_session(conn: &rusqlite::Connection, token: &str) -> Option<(i64, bool)> {
    // Tokens are encrypted with a fresh random nonce each time, so the stored
    // ciphertext cannot be looked up by re-encrypting the plaintext. Instead,
    // decrypt active session rows and compare the original high-entropy token.
    let mut stmt = conn
        .prepare(
            "SELECT admin_id, session_token, is_locked
             FROM admin_sessions
             WHERE expires_at > datetime('now')
             ORDER BY id DESC",
        )
        .ok()?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .ok()?;

    for row in rows.flatten() {
        if let Ok(stored_token) = session::decrypt_token(&row.1) {
            if stored_token == token {
                return Some((row.0, row.2 == 1));
            }
        }
    }

    None
}

fn require_active_session(conn: &rusqlite::Connection, token: &str) -> Result<i64, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Authentication required. Please sign in again.".to_string());
    }
    let (admin_id, locked) = find_session(conn, token)
        .ok_or_else(|| "Session expired or not found. Please sign in again.".to_string())?;
    if locked {
        return Err("Session is locked. Unlock the workspace first.".to_string());
    }
    conn.execute(
        "UPDATE admin_sessions SET last_activity = datetime('now') WHERE admin_id = ?1 AND expires_at > datetime('now') AND is_locked = 0",
        [admin_id],
    ).map_err(|e| format!("Unable to refresh session: {e}"))?;
    Ok(admin_id)
}

#[tauri::command]
pub fn get_auth_status(db: State<Database>) -> AuthStatus {
    let conn = db.conn.lock().unwrap();

    let exists = admin_exists(&conn);

    if !exists {
        return AuthStatus {
            is_setup: false,
            is_authenticated: false,
            is_locked: false,
            admin_username: None,
            token: None,
        };
    }

    let username: Option<String> = conn
        .query_row("SELECT username FROM admin_users LIMIT 1", [], |row| row.get(0))
        .ok();

    let active_session: Option<(i64, String, bool)> = conn
        .query_row(
            "SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)? == 1)),
        )
        .ok();

    let (is_authenticated, is_locked, token) = match active_session {
        Some((_, encrypted_token, locked)) => {
            let token = session::decrypt_token(&encrypted_token).ok();
            (token.is_some(), locked, token)
        }
        None => (false, false, None),
    };

    AuthStatus {
        is_setup: true,
        is_authenticated,
        is_locked,
        admin_username: username,
        token,
    }
}

fn generate_recovery_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[tauri::command]
pub fn setup_admin(db: State<Database>, request: SetupRequest) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    if admin_exists(&conn) {
        return AuthResponse {
            success: false,
            message: "Admin already exists".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    if request.username.trim().len() < 3 {
        return AuthResponse {
            success: false,
            message: "Username must be at least 3 characters".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    if request.password.len() < 8 {
        return AuthResponse {
            success: false,
            message: "Password must be at least 8 characters".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    let recovery_code = generate_recovery_code();
    let recovery_hash = match password::hash_password(&recovery_code) {
        Ok(h) => h,
        Err(e) => {
            return AuthResponse {
                success: false,
                message: e,
                token: None,
                recovery_code: None,
            };
        }
    };

    let hash = match password::hash_password(&request.password) {
        Ok(h) => h,
        Err(e) => {
            return AuthResponse {
                success: false,
                message: e,
                token: None,
                recovery_code: None,
            }
        }
    };

    match conn.execute(
        "INSERT INTO admin_users (username, password_hash, recovery_hash) VALUES (?1, ?2, ?3)",
        rusqlite::params![request.username.trim(), hash, recovery_hash],
    ) {
        Ok(_) => {
            let admin_id = conn.last_insert_rowid();
            audit::log(&conn, "admin_setup", Some("admin_users"), Some(admin_id), None);

            let timeout: i64 = get_setting(&conn, "session_timeout_minutes")
                .and_then(|v| v.parse().ok())
                .unwrap_or(480);

            match create_session(&conn, admin_id, timeout) {
                Ok(token) => AuthResponse {
                    success: true,
                    message: "Setup complete".to_string(),
                    token: Some(token),
                    recovery_code: Some(recovery_code.clone()),
                },
                Err(e) => AuthResponse {
                    success: false,
                    message: e,
                    token: None,
                    recovery_code: None,
                },
            }
        }
        Err(e) => AuthResponse {
            success: false,
            message: format!("Database error: {}", e),
            token: None,
            recovery_code: None,
        },
    }
}

#[tauri::command]
pub fn login(db: State<Database>, request: LoginRequest) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    let admin = match find_admin_by_username(&conn, &request.username) {
        Some(a) => a,
        None => {
            audit::log(
                &conn,
                "login_failed",
                None,
                None,
                Some(&format!("username: {}", request.username)),
            );
            return AuthResponse {
                success: false,
                message: "Invalid credentials".to_string(),
                token: None,
                recovery_code: None,
            };
        }
    };

    let (admin_id, _username, hash) = admin;

    if !password::verify_password(&request.password, &hash) {
        audit::log(
            &conn,
            "login_failed",
            Some("admin_users"),
            Some(admin_id),
            None,
        );
        return AuthResponse {
            success: false,
            message: "Invalid credentials".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    let timeout: i64 = get_setting(&conn, "session_timeout_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(480);

    match create_session(&conn, admin_id, timeout) {
        Ok(token) => {
            audit::log(&conn, "login_success", Some("admin_users"), Some(admin_id), None);
            AuthResponse {
                success: true,
                message: "Login successful".to_string(),
                token: Some(token),
                recovery_code: None,
            }
        }
        Err(e) => AuthResponse {
            success: false,
            message: e,
            token: None,
            recovery_code: None,
        },
    }
}

#[tauri::command]
pub fn logout(db: State<Database>, token: String) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    let admin_id = find_session(&conn, &token).map(|(id, _)| id);

    if let Some(id) = admin_id {
        audit::log(&conn, "logout", Some("admin_users"), Some(id), None);
        // Version 1 is single-admin: terminate every session for this admin.
        let _ = conn.execute("DELETE FROM admin_sessions WHERE admin_id = ?1", [id]);
    }

    AuthResponse {
        success: true,
        message: "Logged out".to_string(),
        token: None,
        recovery_code: None,
    }
}

#[tauri::command]
pub fn lock_session(db: State<Database>, token: String) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    let admin_id = find_session(&conn, &token).map(|(id, _)| id);

    // Version 1 is single-admin. Lock every active session for the identified
    // admin so AuthGate cannot restore an older unlocked session.
    if let Some(id) = admin_id {
        let _ = conn.execute(
            "UPDATE admin_sessions SET is_locked = 1 WHERE admin_id = ?1 AND expires_at > datetime('now')",
            [id],
        );
        audit::log(&conn, "session_locked", Some("admin_users"), Some(id), None);
    }

    AuthResponse {
        success: true,
        message: "Session locked".to_string(),
        token: None,
        recovery_code: None,
    }
}

#[tauri::command]
pub fn unlock_session(db: State<Database>, token: String, password: String) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    // Normally the renderer sends the active session token.
    // If the renderer lost it while the app was locked/reloaded, V1 is
    // single-admin, so recover the newest active locked session and verify
    // the Admin password before unlocking it.
    let session_info: Option<(i64, String)> = if !token.trim().is_empty() {
        find_session(&conn, &token)
            .and_then(|(admin_id, locked)| {
                if locked {
                    Some((admin_id, token.clone()))
                } else {
                    None
                }
            })
    } else {
        conn.query_row(
            "SELECT admin_id, session_token
             FROM admin_sessions
             WHERE is_locked = 1
               AND expires_at > datetime('now')
             ORDER BY id DESC
             LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .ok()
        .and_then(|(admin_id, encrypted_token)| {
            session::decrypt_token(&encrypted_token)
                .ok()
                .map(|plain| (admin_id, plain))
        })
    };

    let (admin_id, active_token) = match session_info {
        Some(value) => value,
        None => {
            return AuthResponse {
                success: false,
                message: "No locked session found".to_string(),
                token: None,
                recovery_code: None,
            };
        }
    };

    let hash: String = match conn.query_row(
        "SELECT password_hash FROM admin_users WHERE id = ?1",
        [admin_id],
        |row| row.get(0),
    ) {
        Ok(value) => value,
        Err(_) => {
            return AuthResponse {
                success: false,
                message: "Admin account not found".to_string(),
                token: None,
                recovery_code: None,
            };
        }
    };

    if !password::verify_password(&password, &hash) {
        audit::log(
            &conn,
            "unlock_failed",
            Some("admin_users"),
            Some(admin_id),
            None,
        );

        return AuthResponse {
            success: false,
            message: "Invalid password".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    // Unlock the newest locked session belonging to this admin.
    let updated = conn.execute(
        "UPDATE admin_sessions
         SET is_locked = 0, last_activity = datetime('now')
         WHERE id = (
             SELECT id
             FROM admin_sessions
             WHERE admin_id = ?1
               AND is_locked = 1
               AND expires_at > datetime('now')
             ORDER BY id DESC
             LIMIT 1
         )",
        [admin_id],
    );

    if updated.is_err() {
        return AuthResponse {
            success: false,
            message: "Unable to unlock session".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    audit::log(
        &conn,
        "session_unlocked",
        Some("admin_users"),
        Some(admin_id),
        None,
    );

    AuthResponse {
        success: true,
        message: "Unlocked".to_string(),
        token: Some(active_token),
        recovery_code: None,
    }
}


#[tauri::command]
pub fn get_admin_profile(db: State<Database>, token: String) -> Result<serde_json::Value, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    let admin_id = match find_session(&conn, &token) {
        Some((id, false)) => id,
        Some((_id, true)) => return Err("Session is locked".into()),
        None => return Err("Session not found".into()),
    };
    let profile = conn.query_row(
        "SELECT id, username, created_at, updated_at FROM admin_users WHERE id=?1",
        [admin_id],
        |r| Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "username": r.get::<_, String>(1)?,
            "created_at": r.get::<_, String>(2)?,
            "updated_at": r.get::<_, String>(3)?
        }))
    ).map_err(|e| format!("Unable to load admin profile: {e}"))?;
    Ok(profile)
}

#[tauri::command]
pub fn update_admin_username(
    db: State<Database>,
    token: String,
    current_password: String,
    new_username: String,
) -> AuthResponse {
    let conn = db.conn.lock().unwrap();
    let admin_id = match require_active_session(&conn, &token) {
        Ok(id) => id,
        Err(message) => return AuthResponse { success: false, message, token: None, recovery_code: None },
    };

    let username = new_username.trim();
    if username.len() < 3 || username.len() > 50 {
        return AuthResponse { success: false, message: "Username must be between 3 and 50 characters".into(), token: None, recovery_code: None };
    }
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return AuthResponse { success: false, message: "Username may contain only letters, numbers, dot, underscore and hyphen".into(), token: None, recovery_code: None };
    }

    let password_hash: String = match conn.query_row(
        "SELECT password_hash FROM admin_users WHERE id=?1", [admin_id], |r| r.get(0)
    ) {
        Ok(v) => v,
        Err(e) => return AuthResponse { success: false, message: format!("Unable to load administrator account: {e}"), token: None, recovery_code: None },
    };
    if !password::verify_password(&current_password, &password_hash) {
        audit::log(&conn, "username_change_failed", Some("admin_users"), Some(admin_id), Some("Current password is incorrect"));
        return AuthResponse { success: false, message: "Current password is incorrect".into(), token: None, recovery_code: None };
    }

    let duplicate: Option<i64> = conn.query_row(
        "SELECT id FROM admin_users WHERE lower(username)=lower(?1) AND id<>?2 LIMIT 1",
        rusqlite::params![username, admin_id],
        |r| r.get(0),
    ).optional().unwrap_or(None);
    if duplicate.is_some() {
        return AuthResponse { success: false, message: "That username is already in use".into(), token: None, recovery_code: None };
    }

    if let Err(e) = conn.execute(
        "UPDATE admin_users SET username=?1, updated_at=datetime('now') WHERE id=?2",
        rusqlite::params![username, admin_id],
    ) {
        return AuthResponse { success: false, message: format!("Unable to update username: {e}"), token: None, recovery_code: None };
    }

    audit::log(&conn, "username_changed", Some("admin_users"), Some(admin_id), Some("Administrator username changed after password verification"));
    AuthResponse { success: true, message: "Username updated successfully".into(), token: Some(token), recovery_code: None }
}

#[tauri::command]
pub fn change_password(
    db: State<Database>,
    token: String,
    current_password: String,
    new_password: String,
) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    let admin_id = match require_active_session(&conn, &token) {
        Ok(id) => id,
        Err(message) => return AuthResponse { success: false, message, token: None, recovery_code: None },
    };

    let hash: String = match conn
        .query_row(
            "SELECT password_hash FROM admin_users WHERE id = ?1",
            [admin_id],
            |row| row.get(0),
        ) {
            Ok(value) => value,
            Err(e) => return AuthResponse { success: false, message: format!("Unable to load administrator account: {e}"), token: None, recovery_code: None },
        };

    if !password::verify_password(&current_password, &hash) {
        audit::log(&conn, "password_change_failed", Some("admin_users"), Some(admin_id), Some("Current password is incorrect"));
        return AuthResponse {
            success: false,
            message: "Current password is incorrect".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    if new_password.len() < 8 {
        return AuthResponse {
            success: false,
            message: "New password must be at least 8 characters".to_string(),
            token: None,
            recovery_code: None,
        };
    }

    let new_hash = match password::hash_password(&new_password) {
        Ok(h) => h,
        Err(e) => {
            return AuthResponse {
                success: false,
                message: e,
                token: None,
                recovery_code: None,
            };
        }
    };

    // A password change invalidates the previous recovery secret. Generate a
    // fresh recovery key atomically with the new password so the old key can
    // never recover the new password. The plaintext key is returned once.
    let recovery_code = generate_recovery_code();
    let recovery_hash = match password::hash_password(&recovery_code) {
        Ok(h) => h,
        Err(e) => {
            return AuthResponse {
                success: false,
                message: e,
                token: None,
                recovery_code: None,
            };
        }
    };

    if let Err(e) = conn.execute(
        "UPDATE admin_users SET password_hash = ?1, recovery_hash = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![new_hash, recovery_hash, admin_id],
    ) {
        return AuthResponse {
            success: false,
            message: e.to_string(),
            token: None,
            recovery_code: None,
        };
    }

    audit::log(&conn, "password_changed", Some("admin_users"), Some(admin_id), None);

    AuthResponse {
        success: true,
        message: "Password changed and recovery key regenerated".to_string(),
        token: None,
        recovery_code: Some(recovery_code),
    }
}

#[tauri::command]
pub fn forgot_password(db: State<Database>, request: ForgotPasswordRequest) -> AuthResponse {
    let conn = db.conn.lock().unwrap();
    let username = request.username.trim();

    let admin = conn
        .query_row(
            "SELECT id, recovery_hash FROM admin_users WHERE username = ?1",
            [username],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .ok();

    let (admin_id, recovery_hash) = match admin {
        Some(value) => value,
        None => {
            return AuthResponse { success: false, message: "Recovery details are incorrect".to_string(), token: None, recovery_code: None };
        }
    };

    let recovery_hash = match recovery_hash {
        Some(value) => value,
        None => {
            return AuthResponse { success: false, message: "Recovery is not configured for this account".to_string(), token: None, recovery_code: None };
        }
    };

    if !password::verify_password(&request.recovery_code.trim().to_uppercase(), &recovery_hash) {
        audit::log(&conn, "password_reset_failed", Some("admin_users"), Some(admin_id), None);
        return AuthResponse { success: false, message: "Recovery details are incorrect".to_string(), token: None, recovery_code: None };
    }

    if request.new_password.len() < 8 {
        return AuthResponse { success: false, message: "New password must be at least 8 characters".to_string(), token: None, recovery_code: None };
    }

    let new_hash = match password::hash_password(&request.new_password) {
        Ok(h) => h,
        Err(e) => {
            return AuthResponse { success: false, message: e, token: None, recovery_code: None };
        }
    };

    if let Err(e) = conn.execute(
        "UPDATE admin_users SET password_hash = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_hash, admin_id],
    ) {
        return AuthResponse { success: false, message: format!("Database error: {}", e), token: None, recovery_code: None };
    }

    let _ = conn.execute("DELETE FROM admin_sessions WHERE admin_id = ?1", [admin_id]);
    audit::log(&conn, "password_reset", Some("admin_users"), Some(admin_id), None);

    AuthResponse { success: true, message: "Password reset successfully".to_string(), token: None, recovery_code: None }
}

#[tauri::command]
pub fn get_security_settings(db: State<Database>) -> serde_json::Value {
    let conn = db.conn.lock().unwrap();

    let auto_lock: i64 = get_setting(&conn, "auto_lock_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);
    let session_timeout: i64 = get_setting(&conn, "session_timeout_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(480);

    serde_json::json!({
        "auto_lock_minutes": auto_lock,
        "session_timeout_minutes": session_timeout,
    })
}

#[tauri::command]
pub fn update_security_settings(
    db: State<Database>,
    token: String,
    auto_lock_minutes: i64,
    session_timeout_minutes: i64,
) -> AuthResponse {
    let conn = db.conn.lock().unwrap();

    let admin_id = match require_active_session(&conn, &token) {
        Ok(id) => id,
        Err(message) => return AuthResponse { success: false, message, token: None, recovery_code: None },
    };

    if auto_lock_minutes < 0 || auto_lock_minutes > 10080 {
        return AuthResponse { success: false, message: "Automatic lock must be between 0 and 10080 minutes".to_string(), token: None, recovery_code: None };
    }
    if session_timeout_minutes < 1 || session_timeout_minutes > 43200 {
        return AuthResponse { success: false, message: "Session timeout must be between 1 and 43200 minutes".to_string(), token: None, recovery_code: None };
    }
    if auto_lock_minutes > session_timeout_minutes {
        return AuthResponse { success: false, message: "Automatic lock cannot be longer than the session timeout".to_string(), token: None, recovery_code: None };
    }
    if let Err(e) = conn.execute_batch("BEGIN IMMEDIATE") {
        return AuthResponse { success: false, message: format!("Unable to save security settings: {e}"), token: None, recovery_code: None };
    }
    let result = (|| -> Result<(), String> {
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('auto_lock_minutes', ?1, datetime('now')) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            [auto_lock_minutes.to_string()],
        ).map_err(|e| format!("Unable to save automatic lock: {e}"))?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('session_timeout_minutes', ?1, datetime('now')) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            [session_timeout_minutes.to_string()],
        ).map_err(|e| format!("Unable to save session timeout: {e}"))?;
        conn.execute(
            "UPDATE admin_sessions SET expires_at = datetime('now', '+' || ?1 || ' minutes'), last_activity = datetime('now') WHERE admin_id = ?2 AND is_locked = 0 AND expires_at > datetime('now')",
            rusqlite::params![session_timeout_minutes, admin_id],
        ).map_err(|e| format!("Unable to apply session timeout: {e}"))?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = conn.execute_batch("ROLLBACK");
        return AuthResponse { success: false, message: e, token: None, recovery_code: None };
    }
    if let Err(e) = conn.execute_batch("COMMIT") {
        let _ = conn.execute_batch("ROLLBACK");
        return AuthResponse { success: false, message: format!("Unable to finalize security settings: {e}"), token: None, recovery_code: None };
    }

    audit::log(
        &conn,
        "security_settings_changed",
        Some("app_settings"),
        Some(admin_id),
        Some(&format!(
            "auto_lock={}min, session_timeout={}min",
            auto_lock_minutes, session_timeout_minutes
        )),
    );

    AuthResponse {
        success: true,
        message: "Settings updated".to_string(),
        token: None,
        recovery_code: None,
    }
}
