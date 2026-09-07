use crate::database::connection::Database;
use crate::security::{audit, session};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: i64,
    pub title: String,
    pub message: Option<String>,
    pub reminder_type: String,
    pub due_date: String,
    pub recurrence: String,
    pub is_completed: bool,
    pub snoozed_until: Option<String>,
    pub completed_at: Option<String>,
    pub read_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub id: Option<i64>,
    pub title: String,
    pub message: Option<String>,
    pub reminder_type: String,
    pub due_date: String,
    pub recurrence: String,
}

fn admin_id(conn: &Connection, token: &str) -> Result<i64, String> {
    let mut stmt = conn.prepare(
        "SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))
        .map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if row.2 == 0 && session::decrypt_token(&row.1).ok().as_deref() == Some(token) {
            return Ok(row.0);
        }
    }
    Err("Session not found or locked".into())
}

fn valid_type(t: &str) -> bool {
    matches!(t, "general" | "payroll" | "leave" | "loan" | "backup" | "update")
}
fn valid_recurrence(r: &str) -> bool { matches!(r, "none" | "daily" | "weekly" | "monthly") }

fn row_to_reminder(r: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: r.get(0)?, title: r.get(1)?, message: r.get(2)?, reminder_type: r.get(3)?, due_date: r.get(4)?,
        recurrence: r.get(5)?, is_completed: r.get::<_, i64>(6)? == 1, snoozed_until: r.get(7)?,
        completed_at: r.get(8)?, read_at: r.get(9)?, created_at: r.get(10)?, updated_at: r.get(11)?,
    })
}

#[tauri::command]
pub fn get_reminders(db: State<Database>, token: String) -> Result<Vec<Reminder>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let mut stmt = conn.prepare("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders ORDER BY is_completed ASC, due_date ASC, id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_reminder).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_reminder(db: State<Database>, token: String, reminder: ReminderInput) -> Result<Reminder, String> {
    if reminder.title.trim().is_empty() { return Err("Reminder title is required.".into()); }
    if !valid_type(&reminder.reminder_type) { return Err("Invalid reminder type.".into()); }
    if !valid_recurrence(&reminder.recurrence) { return Err("Invalid recurrence.".into()); }
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let id = match reminder.id {
        Some(id) => {
            conn.execute("UPDATE reminders SET title=?1,message=?2,reminder_type=?3,due_date=?4,recurrence=?5,updated_at=datetime('now'),read_at=NULL WHERE id=?6", params![reminder.title.trim(), reminder.message, reminder.reminder_type, reminder.due_date, reminder.recurrence, id]).map_err(|e| e.to_string())?;
            audit::log(&conn, "reminder_updated", Some("reminders"), Some(id), None); id
        }
        None => {
            conn.execute("INSERT INTO reminders(title,message,reminder_type,due_date,recurrence,read_at) VALUES(?1,?2,?3,?4,?5,NULL)", params![reminder.title.trim(), reminder.message, reminder.reminder_type, reminder.due_date, reminder.recurrence]).map_err(|e| e.to_string())?;
            let id = conn.last_insert_rowid();
            audit::log(&conn, "reminder_created", Some("reminders"), Some(id), None); id
        }
    };
    conn.query_row("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE id=?1", [id], row_to_reminder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_reminder(db: State<Database>, token: String, reminder_id: i64) -> Result<Reminder, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let recurrence: String = conn.query_row("SELECT recurrence FROM reminders WHERE id=?1", [reminder_id], |r| r.get(0)).map_err(|_| "Reminder not found.")?;
    let auto_advance = conn.query_row("SELECT value FROM app_settings WHERE key='reminder_auto_advance_recurring'", [], |r| r.get::<_,String>(0)).map(|v| v == "1").unwrap_or(true);
    if recurrence == "none" || !auto_advance {
        conn.execute("UPDATE reminders SET is_completed=1,completed_at=datetime('now'),updated_at=datetime('now') WHERE id=?1", [reminder_id]).map_err(|e| e.to_string())?;
    } else {
        let modifier = match recurrence.as_str() { "daily" => "+1 day", "weekly" => "+7 days", "monthly" => "+1 month", _ => "+1 day" };
        conn.execute("UPDATE reminders SET due_date=datetime(due_date,?1),is_completed=0,snoozed_until=NULL,read_at=NULL,updated_at=datetime('now') WHERE id=?2", params![modifier, reminder_id]).map_err(|e| e.to_string())?;
    }
    audit::log(&conn, "reminder_completed", Some("reminders"), Some(reminder_id), None);
    conn.query_row("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE id=?1", [reminder_id], row_to_reminder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn snooze_reminder(db: State<Database>, token: String, reminder_id: i64, minutes: i64) -> Result<Reminder, String> {
    if !(1..=10080).contains(&minutes) { return Err("Snooze time must be between 1 minute and 7 days.".into()); }
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    conn.execute("UPDATE reminders SET snoozed_until=datetime('now','+' || ?1 || ' minutes'),updated_at=datetime('now') WHERE id=?2", params![minutes, reminder_id]).map_err(|e| e.to_string())?;
    audit::log(&conn, "reminder_snoozed", Some("reminders"), Some(reminder_id), Some(&format!("minutes={minutes}")));
    conn.query_row("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE id=?1", [reminder_id], row_to_reminder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_reminder(db: State<Database>, token: String, reminder_id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let changed = conn.execute("DELETE FROM reminders WHERE id=?1", [reminder_id]).map_err(|e| e.to_string())?;
    if changed == 0 { return Err("Reminder not found.".into()); }
    audit::log(&conn, "reminder_deleted", Some("reminders"), Some(reminder_id), None);
    Ok(())
}

#[tauri::command]
pub fn get_unread_due_reminders(db: State<Database>, token: String) -> Result<Vec<Reminder>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let mut stmt = conn.prepare("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE is_completed=0 AND read_at IS NULL AND datetime(due_date) <= datetime('now') AND (snoozed_until IS NULL OR datetime(snoozed_until) <= datetime('now')) ORDER BY due_date ASC, id ASC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_reminder).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mark_reminder_read(db: State<Database>, token: String, reminder_id: i64) -> Result<Reminder, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let changed = conn.execute("UPDATE reminders SET read_at=datetime('now'),updated_at=datetime('now') WHERE id=?1", [reminder_id]).map_err(|e| e.to_string())?;
    if changed == 0 { return Err("Reminder not found.".into()); }
    audit::log(&conn, "reminder_read", Some("reminders"), Some(reminder_id), None);
    conn.query_row("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE id=?1", [reminder_id], row_to_reminder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_due_reminders(db: State<Database>, token: String) -> Result<Vec<Reminder>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let mut stmt = conn.prepare("SELECT id,title,message,reminder_type,due_date,recurrence,is_completed,snoozed_until,completed_at,read_at,created_at,updated_at FROM reminders WHERE is_completed=0 AND datetime(due_date) <= datetime('now') AND (snoozed_until IS NULL OR datetime(snoozed_until) <= datetime('now')) ORDER BY due_date ASC, id ASC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_reminder).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
