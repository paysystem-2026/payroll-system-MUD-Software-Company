use std::{thread, time::Duration};
use rusqlite::Connection;
use super::engine::{backup_settings, db_path_for_scheduler, run_automatic_backup_with_connection};

fn scheduled_backup_due(conn: &Connection, frequency: &str, scheduled_time: &str) -> bool {
    let (today, now_time): (String, String) = match conn.query_row(
        "SELECT strftime('%Y-%m-%d', 'now', 'localtime'),
                strftime('%H:%M', 'now', 'localtime')",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // The scheduler polls frequently, so use a >= comparison instead of
    // requiring an exact minute match. This also catches up if the app was
    // closed when the scheduled minute passed.
    if now_time.as_str() < scheduled_time {
        return false;
    }

    let last_date: Option<String> = conn.query_row(
        "SELECT strftime('%Y-%m-%d', created_at, 'localtime')
         FROM backups
         WHERE backup_type = 'automatic' AND status = 'completed'
         ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get(0),
    ).ok();

    match last_date {
        None => true,
        Some(date) if frequency == "weekly" => {
            let days_since: i64 = conn.query_row(
                "SELECT CAST(julianday(date('now','localtime')) - julianday(?1) AS INTEGER)",
                [date],
                |r| r.get(0),
            ).unwrap_or(0);
            days_since >= 7
        }
        Some(date) => date != today,
    }
}

pub fn start_scheduler() {
    thread::spawn(|| {
        // Give the Tauri/SQLite initialization a moment to settle before the
        // first scheduler cycle. The loop then keeps running independently
        // of the UI and retries failed automatic backups on later cycles.
        thread::sleep(Duration::from_secs(2));
        loop {
            let db = db_path_for_scheduler();
            if let Ok(conn) = Connection::open(&db) {
                let _ = conn.busy_timeout(Duration::from_secs(10));
                let _ = conn.execute_batch("PRAGMA foreign_keys=ON;");

                let (enabled, frequency, time, _retention, _) = backup_settings(&conn);
                if enabled && scheduled_backup_due(&conn, &frequency, &time) {
                    let _ = run_automatic_backup_with_connection(&conn);
                }
            }

            // Ten seconds gives reliable scheduling without requiring an
            // exact clock-minute match.
            thread::sleep(Duration::from_secs(10));
        }
    });
}
