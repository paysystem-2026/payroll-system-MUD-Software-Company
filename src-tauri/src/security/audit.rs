use rusqlite::Connection;

pub fn log(
    conn: &Connection,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<i64>,
    details: Option<&str>,
) {
    let _ = conn.execute(
        "INSERT INTO audit_logs (action, entity_type, entity_id, details) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![action, entity_type, entity_id, details],
    );
}
