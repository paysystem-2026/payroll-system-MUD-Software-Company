use crate::database::connection::Database;
use crate::security::session;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: i64,
    pub title: String,
    pub message: String,
    pub kind: String,
    pub route: String,
    pub route_query: Option<String>,
    pub created_at: String,
    pub read: bool,
}

fn admin_id(conn: &Connection, token: &str) -> Result<i64, String> {
    let mut stmt = conn.prepare("SELECT admin_id,session_token,is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?))).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if row.2 == 0 && session::decrypt_token(&row.1).ok().as_deref() == Some(token) { return Ok(row.0); }
    }
    Err("Session not found or locked".into())
}

fn map_event(action: &str, entity: Option<&str>, entity_id: Option<i64>, details: Option<&str>) -> Option<(String,String,String,Option<String>)> {
    let (title, kind, route, query) = match action {
        "employee_created" | "employee_updated" => ("Staff record updated", "staff", "/staff", entity_id.map(|id| format!("employee={id}"))),
        "employee_deleted" => ("Employee removed", "staff", "/staff", None),
        "salary_added" => ("Salary record added", "staff", "/staff", None),
        "leave_created" | "leave_updated" => ("Leave record updated", "leave", "/leaves", entity_id.map(|id| format!("leave={id}"))),
        "leave_status_changed" => ("Leave status changed", "leave", "/leaves", entity_id.map(|id| format!("leave={id}"))),
        "payroll_period_created" => ("Payroll period created", "payroll", "/payroll", entity_id.map(|id| format!("period={id}"))),
        "payroll_calculation_completed" => ("Payroll calculation completed", "payroll", "/payroll", entity_id.map(|id| format!("period={id}"))),
        "payroll_closed" => ("Payroll period closed", "payroll", "/payroll", entity_id.map(|id| format!("period={id}"))),
        "loan_created" | "loan_updated" => ("Loan / advance updated", "loan", "/payroll", entity_id.map(|id| format!("loan={id}"))),
        "backup_created" => ("Backup completed", "backup", "/backup", entity_id.map(|id| format!("backup={id}"))),
        "backup_failed" => ("Backup failed", "backup", "/backup", None),
        "transfer_completed" | "transfer_failed" => ("LAN transfer event", "transfer", "/lan-transfer", None),
        "update_available" => ("System update available", "update", "/updates", None),
        "update_install_failed" => ("System update failed", "update", "/updates", None),
        _ => return None,
    };
    let msg = details.unwrap_or(title).to_string();
    let _ = entity;
    Some((title.to_string(), msg, kind.to_string(), query.map(|q| format!("{route}?{q}"))))
}

fn reminder_rows(conn: &Connection, limit: i64) -> Result<Vec<Notification>, String> {
    let mut stmt = conn.prepare("SELECT id,title,COALESCE(message,'Reminder is due.'),due_date FROM reminders WHERE is_completed=0 AND read_at IS NULL AND datetime(due_date)<=datetime('now') AND (snoozed_until IS NULL OR datetime(snoozed_until)<=datetime('now')) ORDER BY due_date ASC,id ASC LIMIT ?1").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([limit], |r| Ok(Notification {
        id: -r.get::<_,i64>(0)?, title: r.get(1)?, message: r.get(2)?, kind: "reminder".into(), route: "/reminders".into(), route_query: Some(format!("reminder={}", r.get::<_,i64>(0)?)), created_at: r.get(3)?, read: false,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_notifications(db: State<Database>, token: String) -> Result<Vec<Notification>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    let notifications_enabled = conn.query_row("SELECT value FROM app_settings WHERE key='reminder_notifications_enabled'", [], |r| r.get::<_,String>(0)).map(|v| v == "1").unwrap_or(true);
    let mut out = if notifications_enabled { reminder_rows(&conn, 20)? } else { Vec::new() };
    let mut stmt = conn.prepare("SELECT a.id,a.action,a.entity_type,a.entity_id,a.details,a.created_at,CASE WHEN nr.audit_id IS NULL THEN 0 ELSE 1 END FROM audit_logs a LEFT JOIN notification_reads nr ON nr.audit_id=a.id WHERE nr.audit_id IS NULL ORDER BY a.id DESC LIMIT 60").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,Option<String>>(2)?, r.get::<_,Option<i64>>(3)?, r.get::<_,Option<String>>(4)?, r.get::<_,String>(5)?))
    }).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if let Some((title,msg,kind,query)) = map_event(&row.1,row.2.as_deref(),row.3,row.4.as_deref()) {
            out.push(Notification{id:row.0,title,message:msg,kind,route:query.as_deref().and_then(|q|q.split('?').next()).unwrap_or("/").to_string(),route_query:query.and_then(|q|q.split_once('?').map(|(_,v)|v.to_string())),created_at:row.5,read:false});
        }
    }
    out.sort_by(|a,b| b.created_at.cmp(&a.created_at));
    out.truncate(30);
    Ok(out)
}

#[tauri::command]
pub fn mark_notification_read(db: State<Database>, token: String, notification_id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    if notification_id < 0 {
        conn.execute("UPDATE reminders SET read_at=datetime('now'),updated_at=datetime('now') WHERE id=?1", [-notification_id]).map_err(|e| e.to_string())?;
    } else {
        conn.execute("INSERT OR IGNORE INTO notification_reads(audit_id,read_at) VALUES(?1,datetime('now'))", [notification_id]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn mark_all_notifications_read(db: State<Database>, token: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|_| "Database lock failed".to_string())?;
    admin_id(&conn, &token)?;
    conn.execute("UPDATE reminders SET read_at=datetime('now'),updated_at=datetime('now') WHERE is_completed=0 AND read_at IS NULL AND datetime(due_date)<=datetime('now') AND (snoozed_until IS NULL OR datetime(snoozed_until)<=datetime('now'))", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO notification_reads(audit_id,read_at) SELECT a.id,datetime('now') FROM audit_logs a LEFT JOIN notification_reads nr ON nr.audit_id=a.id WHERE nr.audit_id IS NULL", []).map_err(|e| e.to_string())?;
    Ok(())
}
