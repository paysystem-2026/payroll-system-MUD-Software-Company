use rusqlite::{types::ValueRef, Connection};
use serde::Serialize;
use tauri::State;

use crate::database::connection::Database;
use crate::security::session;

#[derive(Serialize, Clone)]
pub struct GlobalSearchResult {
    pub table: String,
    pub record_id: Option<i64>,
    pub title: String,
    pub subtitle: String,
    pub matched_field: String,
    pub matched_value: String,
    pub route: String,
}

fn require_admin(conn: &Connection, token: &str) -> Result<i64, String> {
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

fn quote_identifier(value: &str) -> String { format!("\"{}\"", value.replace('"', "\"\"")) }

fn is_secret_column(column: &str) -> bool {
    let c = column.to_ascii_lowercase();
    ["password", "password_hash", "hash", "session_token", "token", "secret", "private_key", "encryption_key", "recovery_code", "api_key"]
        .iter().any(|x| c == *x || c.contains(x))
}

fn route_for_table(table: &str) -> &'static str {
    match table {
        "employees" | "employee_salary_history" | "employee_components" | "employee_payroll_overrides" | "contract_types" => "/staff",
        "companies" | "departments" | "positions" | "payroll_configurations" | "salary_components" | "payroll_rules" | "payroll_rule_versions" | "payroll_rule_dependencies" => "/basic-data",
        "leave_types" | "leave_records" | "leave_balances" => "/leaves",
        "payroll_periods" | "payroll_records" | "payroll_items" | "loans" | "loan_payments" | "payslips" => "/payroll",
        "reminders" => "/reminders",
        "backups" => "/backup",
        "devices" | "transfer_history" => "/lan-transfer",
        "update_history" => "/updates",
        "audit_logs" | "admin_users" => "/administration",
        "app_settings" => "/settings",
        _ => "/",
    }
}

fn value_to_string(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Null => None,
        ValueRef::Integer(v) => Some(v.to_string()),
        ValueRef::Real(v) => Some(v.to_string()),
        ValueRef::Text(v) => Some(String::from_utf8_lossy(v).into_owned()),
        ValueRef::Blob(_) => None,
    }
}

fn display_title(table: &str, columns: &[String], values: &[Option<String>], fallback: &str) -> String {
    let preferred = match table {
        "employees" => &["first_name", "last_name"][..],
        "departments" => &["name"][..],
        "positions" => &["title", "name"][..],
        "companies" => &["name", "company_name"][..],
        "salary_components" | "payroll_rules" => &["name", "code"][..],
        "payroll_periods" => &["period_name", "name"][..],
        "leave_types" => &["name", "title"][..],
        "reminders" => &["title", "name"][..],
        _ => &["name", "title", "code", "employee_code", "period_name", "action", "key"][..],
    };
    if table == "employees" {
        let first = columns.iter().position(|c| c.eq_ignore_ascii_case("first_name"))
            .and_then(|i| values.get(i).and_then(|v| v.clone())).unwrap_or_default();
        let last = columns.iter().position(|c| c.eq_ignore_ascii_case("last_name"))
            .and_then(|i| values.get(i).and_then(|v| v.clone())).unwrap_or_default();
        let full = format!("{} {}", first.trim(), last.trim()).trim().to_string();
        if !full.is_empty() { return full; }
    }
    for key in preferred {
        if let Some(i) = columns.iter().position(|c| c.eq_ignore_ascii_case(key)) {
            if let Some(v) = values.get(i).and_then(|v| v.clone()) {
                if !v.trim().is_empty() { return v; }
            }
        }
    }
    fallback.to_string()
}

fn display_subtitle(table: &str, columns: &[String], values: &[Option<String>], matched_field: &str) -> String {
    let preferred = match table {
        "employees" => &["employee_code", "phone", "email", "employment_status"][..],
        "payroll_records" => &["employee_id", "period_id", "net_pay", "status"][..],
        "payroll_periods" => &["start_date", "end_date", "status"][..],
        "audit_logs" => &["action", "entity_type", "created_at"][..],
        _ => &["code", "status", "created_at", "updated_at", "description"][..],
    };
    let parts: Vec<String> = preferred.iter().filter_map(|key| {
        if key.eq_ignore_ascii_case(matched_field) { return None; }
        let i = columns.iter().position(|c| c.eq_ignore_ascii_case(key))?;
        let v = values.get(i)?.as_ref()?.trim();
        if v.is_empty() { None } else { Some(v.to_string()) }
    }).take(3).collect();
    if parts.is_empty() { format!("{} · {}", table.replace('_', " "), matched_field.replace('_', " ")) } else { parts.join(" · ") }
}

#[tauri::command]
pub fn global_search(db: State<Database>, token: String, query: String, limit: Option<i64>) -> Result<Vec<GlobalSearchResult>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _ = require_admin(&conn, &token)?;
    let query = query.trim();
    if query.len() < 2 { return Ok(Vec::new()); }
    let limit = limit.unwrap_or(50).clamp(1, 100) as usize;
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let like = format!("%{}%", escaped);
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    let mut tables_stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name <> 'schema_migrations' ORDER BY name")
        .map_err(|e| format!("Unable to inspect search tables: {e}"))?;
    let tables: Vec<String> = tables_stmt.query_map([], |r| r.get(0))
        .map_err(|e| format!("Unable to list search tables: {e}"))?
        .filter_map(Result::ok).collect();

    for table in tables {
        if results.len() >= limit { break; }
        let table_q = quote_identifier(&table);
        let pragma = format!("PRAGMA table_info({})", table_q);
        let mut col_stmt = conn.prepare(&pragma).map_err(|e| format!("Unable to inspect {table}: {e}"))?;
        let columns: Vec<(String, String)> = col_stmt.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
            .map_err(|e| format!("Unable to inspect {table}: {e}"))?
            .filter_map(Result::ok).collect();
        let column_names: Vec<String> = columns.iter().map(|(name, _)| name.clone()).collect();
        let searchable: Vec<String> = columns.into_iter()
            .filter(|(name, ty)| !is_secret_column(name) && !ty.eq_ignore_ascii_case("BLOB"))
            .map(|(name, _)| name)
            .collect();
        if searchable.is_empty() { continue; }

        for column in searchable {
            if results.len() >= limit { break; }
            let col_q = quote_identifier(&column);
            let sql = format!(
                "SELECT rowid, * FROM {table_q} WHERE CAST({col_q} AS TEXT) LIKE ?1 ESCAPE '\\' LIMIT ?2"
            );
            let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => continue };
            let remaining = (limit - results.len()) as i64;
            let mapped = stmt.query_map(rusqlite::params![like, remaining], |row| {
                let rowid = row.get::<_, i64>(0).ok();
                let column_count = row.as_ref().column_count();
                let mut values = Vec::with_capacity(column_count.saturating_sub(1));
                for i in 1..column_count { values.push(value_to_string(row.get_ref(i)?)); }
                Ok((rowid, values))
            }).map_err(|e| format!("Unable to search {table}: {e}"))?;

            for item in mapped.flatten() {
                let key = format!("{}:{}", table, item.0.unwrap_or(-1));
                if !seen.insert(key) { continue; }
                let title = display_title(&table, &column_names, &item.1, &format!("{} #{}", table.replace('_', " "), item.0.unwrap_or_default()));
                let subtitle = display_subtitle(&table, &column_names, &item.1, &column);
                let matched_value = column_names.iter().position(|c| c.eq_ignore_ascii_case(&column))
                    .and_then(|i| item.1.get(i).cloned()).flatten().unwrap_or_else(|| query.to_string());
                results.push(GlobalSearchResult {
                    table: table.clone(),
                    record_id: item.0,
                    title,
                    subtitle,
                    matched_field: column.clone(),
                    matched_value,
                    route: route_for_table(&table).into(),
                });
                if results.len() >= limit { break; }
            }
        }
    }

    Ok(results)
}
