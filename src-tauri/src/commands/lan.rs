use tauri::State;
use crate::core::lan_transfer::{self, LanDevice, PairingRequest, PairingStatus};
use crate::database::connection::Database;

#[tauri::command]
pub fn get_lan_device(db: State<Database>) -> Result<LanDevice, String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    let (device_id,device_name)=lan_transfer::ensure_identity(&conn)?;
    Ok(LanDevice{device_id,device_name,ip_address:"This device".into(),app_version:env!("CARGO_PKG_VERSION").into(),protocol:"PAYROLL_LAN_V1".into()})
}
#[tauri::command]
pub fn discover_lan_devices(db: State<Database>) -> Result<Vec<LanDevice>, String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; let identity=lan_transfer::ensure_identity(&conn)?; drop(conn); lan_transfer::discover(&identity)
}
#[tauri::command]
pub fn request_lan_pairing(db: State<Database>, device: LanDevice) -> Result<serde_json::Value,String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; let identity=lan_transfer::ensure_identity(&conn)?;
    let (code,nonce)=lan_transfer::create_pairing_request(&identity,&device)?;
    Ok(serde_json::json!({"code":code,"nonce":nonce,"expires_seconds":180,"device_id":device.device_id,"device_name":device.device_name}))
}
#[tauri::command]
pub fn get_lan_pairing_requests(db: State<Database>) -> Result<Vec<PairingRequest>,String> { let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; lan_transfer::list_pairing_requests(&conn) }
#[tauri::command]
pub fn approve_lan_pairing(db: State<Database>, device_id:String, code:String) -> Result<PairingStatus,String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; let (local_id,_)=lan_transfer::ensure_identity(&conn)?; lan_transfer::approve_pairing(&conn,&local_id,&device_id,&code)
}
#[tauri::command]
pub fn get_paired_lan_devices(db: State<Database>) -> Result<Vec<LanDevice>,String> { let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; lan_transfer::list_paired(&conn) }

pub fn start_lan_discovery(db: &Database) -> Result<(),String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?; let _=lan_transfer::ensure_identity(&conn)?; drop(conn);
    let db_path=std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_|".".into())).join(".payroll-system").join("payroll.db");
    lan_transfer::start_responder(db_path.clone());
    lan_transfer::start_transfer_responder(db_path); Ok(())
}

#[tauri::command]
pub fn send_lan_backup(db: State<Database>, backup_id: i64, device_id: String) -> Result<(), String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    let (local_id,_)=lan_transfer::ensure_identity(&conn)?;
    lan_transfer::send_backup(&conn,&local_id,&device_id,backup_id)
}

#[tauri::command]
pub fn test_lan_connection(db: State<Database>, device_id: String) -> Result<LanDevice, String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    let (local_id,_)=lan_transfer::ensure_identity(&conn)?;
    lan_transfer::test_connection(&conn,&local_id,&device_id)
}

#[tauri::command]
pub fn revoke_lan_device(db: State<Database>, device_id: String) -> Result<(), String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    lan_transfer::revoke_device(&conn,&device_id)
}

#[tauri::command]
pub fn get_lan_transfer_history(db: State<Database>) -> Result<Vec<serde_json::Value>, String> {
    let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    let mut stmt=conn.prepare("SELECT th.id,th.direction,th.file_name,COALESCE(th.file_size,0),th.status,th.created_at,COALESCE(d.device_name,'Unknown device') FROM transfer_history th LEFT JOIN devices d ON d.id=th.device_id ORDER BY th.id DESC LIMIT 50").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([],|r| Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"direction":r.get::<_,String>(1)?,"file_name":r.get::<_,String>(2)?,"file_size":r.get::<_,i64>(3)?,"status":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?,"device_name":r.get::<_,String>(6)?}))).map_err(|e|e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
pub fn import_lan_backup(db: State<Database>, token: String, backup_id: i64) -> Result<crate::commands::backup::BackupResponse, String> {
    let mut conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;
    // Reuse the same authorization contract as Backup & Restore.
    let authorized = {
        let mut stmt=conn.prepare("SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?))).map_err(|e|e.to_string())?;
        let result = rows.flatten().any(|row| {
            crate::security::session::decrypt_token(&row.1).ok().as_deref() == Some(token.as_str()) && row.2 == 0
        });
        result
    };
    if !authorized { return Err("Invalid or locked session".into()); }
    let backup_type: String=conn.query_row("SELECT backup_type FROM backups WHERE id=?1 AND status='completed'", rusqlite::params![backup_id], |r|r.get(0)).map_err(|_|"LAN backup not found".to_string())?;
    if backup_type != "lan_received" { return Err("Only received LAN backups can be imported here".into()); }
    match crate::core::backup::import_lan_backup(&mut conn, backup_id) {
        Ok(()) => Ok(crate::commands::backup::BackupResponse{success:true,message:"LAN backup imported successfully. A pre-import safety backup was created. Please sign in again to continue.".into(),backup:None}),
        Err(e) => Ok(crate::commands::backup::BackupResponse{success:false,message:e,backup:None}),
    }
}
