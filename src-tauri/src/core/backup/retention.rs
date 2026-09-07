use rusqlite::{params, Connection};
use std::fs;

pub fn apply(conn: &Connection, retention: i64) {
    let keep = retention.max(1);
    let mut stmt = match conn.prepare(
        "SELECT id, file_path FROM backups WHERE backup_type = 'automatic' AND status = 'completed' ORDER BY id DESC",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };
    let rows = match stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))) {
        Ok(r) => r,
        Err(_) => return,
    };
    for (index, row) in rows.flatten().enumerate() {
        if (index as i64) < keep { continue; }
        let _ = fs::remove_file(&row.1);
        let _ = conn.execute("DELETE FROM backups WHERE id = ?1", params![row.0]);
    }
}
