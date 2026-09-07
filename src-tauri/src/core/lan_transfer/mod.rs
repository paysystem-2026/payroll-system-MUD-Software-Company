use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket, TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DISCOVERY_PORT: u16 = 39271;
const MAGIC: &str = "PAYROLL_LAN_V1";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_PAIR_ATTEMPTS: i64 = 5;
const HEALTH_TIMEOUT: Duration = Duration::from_millis(900);
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TRANSFER_SIZE: u64 = 512 * 1024 * 1024;

fn compatible_version(remote: &str) -> bool {
    let local_major = APP_VERSION.split('.').next().unwrap_or("0");
    let remote_major = remote.split('.').next().unwrap_or("0");
    local_major == remote_major
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanDevice {
    pub device_id: String,
    pub device_name: String,
    pub ip_address: String,
    pub app_version: String,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PairingRequest {
    pub device_id: String,
    pub device_name: String,
    pub ip_address: String,
    pub app_version: String,
    pub requested_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PairingStatus {
    pub state: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub message: String,
}

fn device_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "Payroll Device".to_string())
}

pub fn ensure_identity(conn: &Connection) -> Result<(String, String), String> {
    let id: Option<String> = conn.query_row("SELECT value FROM app_settings WHERE key='lan_device_id'", [], |r| r.get(0)).ok();
    let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    conn.execute("INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES('lan_device_id',?1,datetime('now'))", params![id])
        .map_err(|e| e.to_string())?;
    Ok((id, device_name()))
}

fn now_epoch() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }

fn hash_pair_code(code: &str, nonce: &str, a: &str, b: &str) -> String {
    let mut ids = [a, b];
    ids.sort_unstable();
    let mut h = Sha256::new();
    h.update(b"PAYROLL_PAIR_V2|");
    h.update(code.as_bytes()); h.update(b"|"); h.update(nonce.as_bytes()); h.update(b"|");
    h.update(ids[0].as_bytes()); h.update(b"|"); h.update(ids[1].as_bytes());
    STANDARD.encode(h.finalize())
}

fn derive_secret(code: &str, nonce: &str, a: &str, b: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"PAYROLL_TRANSFER_SECRET_V2|");
    h.update(code.as_bytes()); h.update(b"|"); h.update(nonce.as_bytes()); h.update(b"|");
    let mut ids = [a, b]; ids.sort_unstable();
    h.update(ids[0].as_bytes()); h.update(b"|"); h.update(ids[1].as_bytes());
    h.finalize().into()
}

fn encrypt_secret(secret: &[u8; 32]) -> Result<String, String> {
    let key = sha2::Sha256::digest(format!("{}|{}", std::env::var("USER").unwrap_or_default(), "PAYROLL_LOCAL_SECRET_V2").as_bytes());
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Unable to initialize local encryption".to_string())?;
    let mut nonce_bytes = [0u8; 12]; rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce_bytes), secret.as_ref()).map_err(|_| "Unable to protect pairing secret".to_string())?;
    let mut out = nonce_bytes.to_vec(); out.extend(ciphertext); Ok(STANDARD.encode(out))
}

fn decrypt_secret(encoded: &str) -> Result<[u8; 32], String> {
    let raw = STANDARD.decode(encoded).map_err(|_| "Invalid stored pairing secret".to_string())?;
    if raw.len() < 12 + 16 { return Err("Invalid stored pairing secret".into()); }
    let key = sha2::Sha256::digest(format!("{}|{}", std::env::var("USER").unwrap_or_default(), "PAYROLL_LOCAL_SECRET_V2").as_bytes());
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Unable to initialize local encryption".to_string())?;
    let plaintext = cipher.decrypt(Nonce::from_slice(&raw[..12]), &raw[12..]).map_err(|_| "Unable to decrypt pairing secret".to_string())?;
    if plaintext.len() != 32 { return Err("Invalid pairing secret".into()); }
    let mut out = [0u8; 32]; out.copy_from_slice(&plaintext); Ok(out)
}

fn transfer_nonce() -> [u8; 12] { let mut n=[0u8;12]; rand::thread_rng().fill_bytes(&mut n); n }
fn transfer_tag(secret: &[u8;32], transfer_id: &str, sender: &str, filename: &str, size: u64, checksum: &str) -> String {
    let mut h=Sha256::new(); h.update(b"PAYROLL_TRANSFER_AUTH_V1|"); h.update(secret); h.update(b"|"); h.update(transfer_id); h.update(b"|"); h.update(sender); h.update(b"|"); h.update(filename.as_bytes()); h.update(b"|"); h.update(size.to_string().as_bytes()); h.update(b"|"); h.update(checksum.as_bytes()); STANDARD.encode(h.finalize())
}

fn health_tag(secret: &[u8;32], sender: &str, receiver: &str, nonce: &str) -> String {
    let mut h=Sha256::new();
    h.update(b"PAYROLL_HEALTH_V1|"); h.update(secret); h.update(b"|");
    h.update(sender.as_bytes()); h.update(b"|"); h.update(receiver.as_bytes()); h.update(b"|"); h.update(nonce.as_bytes());
    STANDARD.encode(h.finalize())
}
fn chunk_nonce(base: &[u8;12], index: u32) -> [u8;12] { let mut n=*base; n[8..12].copy_from_slice(&index.to_be_bytes()); n }

fn generate_code() -> String { format!("{:08}", rand::thread_rng().next_u32() % 100_000_000) }
fn generate_nonce() -> String { Uuid::new_v4().to_string() }

fn encode_hello(device_id: &str, name: &str) -> String { format!("{}|{}|{}|{}", MAGIC, device_id, APP_VERSION, name.replace('|', " ")) }
fn decode_hello(message: &str, ip: Ipv4Addr) -> Option<LanDevice> {
    let mut parts = message.splitn(4, '|');
    if parts.next()? != MAGIC { return None; }
    let device_id = parts.next()?.trim(); let app_version = parts.next()?.trim(); let device_name = parts.next()?.trim();
    if device_id.is_empty() || app_version.is_empty() || device_name.is_empty() { return None; }
    Some(LanDevice { device_id: device_id.to_string(), device_name: device_name.to_string(), ip_address: ip.to_string(), app_version: app_version.to_string(), protocol: MAGIC.to_string() })
}

pub fn start_responder(db_path: std::path::PathBuf) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT)) { Ok(s) => s, Err(_) => return };
        let _ = socket.set_broadcast(true);
        let mut buffer = [0u8; 2048];
        loop {
            let Ok((size, peer)) = socket.recv_from(&mut buffer) else { continue };
            let Ok(message) = std::str::from_utf8(&buffer[..size]) else { continue };
            let Ok(conn) = Connection::open(&db_path) else { continue };
            let identity = ensure_identity(&conn).ok();
            let Some((local_id, local_name)) = identity else { continue };
            if message.trim() == "PAYROLL_DISCOVER_V1" {
                let response = encode_hello(&local_id, &local_name);
                let _ = socket.send_to(response.as_bytes(), peer);
                continue;
            }
            if let Some(rest) = message.strip_prefix("PAYROLL_PAIR_REQ_V2|") {
                let p: Vec<&str> = rest.split('|').collect();
                if p.len() != 6 || p[0] == local_id { continue; }
                let remote_id = p[0]; let remote_name = p[1].replace('\n', " "); let remote_version = p[2]; let nonce = p[3]; let proof = p[4];
                if !compatible_version(remote_version) || nonce.len() < 16 || proof.len() != 44 { continue; }
                let _ = conn.execute("DELETE FROM devices WHERE status='pending' AND updated_at < datetime('now','-3 minutes')", []);
                let _ = conn.execute("INSERT INTO devices(device_name,device_id,ip_address,pairing_code,status,last_seen,updated_at) VALUES(?1,?2,?3,?4,'pending',datetime('now'),datetime('now')) ON CONFLICT(device_id) DO UPDATE SET device_name=excluded.device_name,ip_address=excluded.ip_address,pairing_code=excluded.pairing_code,status='pending',last_seen=datetime('now'),updated_at=datetime('now')", params![remote_name, remote_id, peer.ip().to_string(), format!("{}|{}|{}|0|{}", nonce, proof, p[5], remote_version)]);
                continue;
            }
            if let Some(rest) = message.strip_prefix("PAYROLL_PAIR_ACCEPT_V2|") {
                let p: Vec<&str> = rest.split('|').collect();
                if p.len() != 4 { continue; }
                let remote_id = p[0]; let nonce = p[1]; let code = p[2]; let proof = p[3];
                let pending: Option<(String, String)> = conn.query_row("SELECT pairing_code,COALESCE(updated_at,'') FROM devices WHERE device_id=?1 AND status='pending'", params![remote_id], |r| Ok((r.get(0)?, r.get(1)?))).ok();
                let Some((raw, updated_at)) = pending else { continue; };
                let q: Vec<&str> = raw.split('|').collect();
                if q.len() < 2 || q[0] != nonce { continue; }
                let fresh: i64 = conn.query_row("SELECT CASE WHEN updated_at >= datetime('now','-3 minutes') THEN 1 ELSE 0 END FROM devices WHERE device_id=?1", params![remote_id], |r| r.get(0)).unwrap_or(0);
                if fresh != 1 || updated_at.is_empty() { continue; }
                if hash_pair_code(code, nonce, &local_id, remote_id) != proof { continue; }
                let secret = derive_secret(code, nonce, &local_id, remote_id);
                if let Ok(encrypted) = encrypt_secret(&secret) {
                    let _ = conn.execute("UPDATE devices SET pairing_code=?1,status='paired',paired_at=datetime('now'),last_seen=datetime('now'),updated_at=datetime('now') WHERE device_id=?2", params![encrypted, remote_id]);
                    let _ = socket.send_to(format!("PAYROLL_PAIR_CONFIRM_V2|{}|{}", local_id, nonce).as_bytes(), peer);
                }
                continue;
            }
            if let Some(rest) = message.strip_prefix("PAYROLL_PAIR_APPROVE_V2|") {
                let p: Vec<&str> = rest.split('|').collect();
                if p.len() != 4 { continue; }
                let remote_id = p[0]; let nonce = p[1]; let proof = p[2]; let code = p[3];
                if hash_pair_code(code, nonce, &local_id, remote_id) != proof { continue; }
                let secret = derive_secret(code, nonce, &local_id, remote_id);
                if let Ok(encrypted) = encrypt_secret(&secret) {
                    let _ = conn.execute("UPDATE devices SET pairing_code=?1,status='paired',paired_at=datetime('now'),last_seen=datetime('now'),updated_at=datetime('now') WHERE device_id=?2", params![encrypted, remote_id]);
                    let _ = socket.send_to(format!("PAYROLL_PAIR_ACCEPT_V2|{}|{}|{}|{}", local_id, nonce, code, proof).as_bytes(), peer);
                }
            }
            if let Some(rest) = message.strip_prefix("PAYROLL_HEALTH_V1|") {
                let p: Vec<&str> = rest.split('|').collect();
                if p.len() != 3 || p[0] == local_id { continue; }
                let remote_id = p[0]; let nonce = p[1]; let auth = p[2];
                let stored: Option<String> = conn.query_row("SELECT pairing_code FROM devices WHERE device_id=?1 AND status='paired'", params![remote_id], |r| r.get(0)).ok();
                let Some(stored) = stored else { continue; };
                let Ok(secret) = decrypt_secret(&stored) else { continue; };
                let expected = health_tag(&secret, remote_id, &local_id, nonce);
                if expected != auth { continue; }
                let _ = conn.execute("UPDATE devices SET last_seen=datetime('now'),updated_at=datetime('now') WHERE device_id=?1", params![remote_id]);
                let response = format!("PAYROLL_HEALTH_OK_V1|{}|{}|{}", local_id, nonce, health_tag(&secret, &local_id, remote_id, nonce));
                let _ = socket.send_to(response.as_bytes(), peer);
            }
        }
    });
}

pub fn start_transfer_responder(db_path: std::path::PathBuf) {
    thread::spawn(move || {
        let listener = match TcpListener::bind((Ipv4Addr::UNSPECIFIED, TRANSFER_PORT)) { Ok(v)=>v, Err(_)=>return };
        for stream in listener.incoming() {
            let Ok(mut stream)=stream else { continue };
            let path=db_path.clone();
            thread::spawn(move || { let _=receive_transfer(&mut stream, &path); });
        }
    });
}

const TRANSFER_PORT: u16 = 39272;

fn receive_transfer(stream: &mut TcpStream, db_path: &std::path::Path) -> Result<(), String> {
    let mut len=[0u8;4]; stream.read_exact(&mut len).map_err(|_|"Transfer connection closed".to_string())?;
    let header_len=u32::from_be_bytes(len) as usize; if header_len>64*1024 { return Err("Invalid transfer header".into()); }
    let mut hb=vec![0u8;header_len]; stream.read_exact(&mut hb).map_err(|_|"Unable to read transfer header".to_string())?;
    let header: serde_json::Value=serde_json::from_slice(&hb).map_err(|_|"Invalid transfer header".to_string())?;
    let sender=header.get("sender_id").and_then(|v|v.as_str()).ok_or("Missing sender")?;
    let filename=header.get("filename").and_then(|v|v.as_str()).ok_or("Missing filename")?;
    let size=header.get("size").and_then(|v|v.as_u64()).ok_or("Missing size")?;
    let checksum=header.get("sha256").and_then(|v|v.as_str()).ok_or("Missing checksum")?;
    let transfer_id=header.get("transfer_id").and_then(|v|v.as_str()).ok_or("Missing transfer id")?;
    let base_b64=header.get("nonce").and_then(|v|v.as_str()).ok_or("Missing nonce")?;
    let base_vec=STANDARD.decode(base_b64).map_err(|_|"Invalid transfer nonce".to_string())?; if base_vec.len()!=12 { return Err("Invalid transfer nonce".into()); }
    let mut base=[0u8;12]; base.copy_from_slice(&base_vec);
    let auth=header.get("auth").and_then(|v|v.as_str()).ok_or("Missing transfer authentication")?;
    let version=header.get("version").and_then(|v|v.as_str()).ok_or("Missing sender version")?;
    if !compatible_version(version) { return Err("Sender is running an incompatible Payroll System version".into()); }
    let conn=Connection::open(db_path).map_err(|e|e.to_string())?;
    let (local_id,_)=ensure_identity(&conn)?;
    if sender==local_id { return Err("Refusing self-transfer".into()); }
    let stored: String=conn.query_row("SELECT pairing_code FROM devices WHERE device_id=?1 AND status='paired'",params![sender],|r|r.get(0)).map_err(|_|"Sender is not a trusted paired device".to_string())?;
    let secret=decrypt_secret(&stored)?;
    if transfer_tag(&secret,transfer_id,sender,filename,size,checksum)!=auth { return Err("Transfer authentication failed".into()); }
    if size>MAX_TRANSFER_SIZE { return Err("Transfer exceeds the 512 MB safety limit".into()); }
    stream.set_read_timeout(Some(TRANSFER_TIMEOUT)).map_err(|e| format!("Unable to configure transfer timeout: {e}"))?;
    stream.set_write_timeout(Some(TRANSFER_TIMEOUT)).map_err(|e| format!("Unable to configure transfer timeout: {e}"))?;
    let safe_name=std::path::Path::new(filename).file_name().and_then(|v|v.to_str()).unwrap_or("received-backup").replace(|c:char| !c.is_ascii_alphanumeric() && c!='.' && c!='-' && c!='_', "_");
    let (_, _, _, _, configured_location)=crate::core::backup::backup_settings(&conn); let out_dir=std::path::PathBuf::from(configured_location); std::fs::create_dir_all(&out_dir).map_err(|e|e.to_string())?;
    let temp=out_dir.join(format!(".lan-{}-{}.part", transfer_id, safe_name));
    let final_path=out_dir.join(format!("lan_received_{}_{}", transfer_id, safe_name));
    if final_path.exists() { return Err("This transfer was already received".into()); }
    let receive_result: Result<(), String> = (|| {
        let mut file=std::fs::File::create(&temp).map_err(|e|e.to_string())?;
        let cipher=Aes256Gcm::new_from_slice(&secret).map_err(|_|"Unable to initialize transfer encryption".to_string())?;
        let mut received=0u64; let mut index=0u32; let mut hasher=Sha256::new();
        while received<size {
            stream.read_exact(&mut len).map_err(|_|"Transfer interrupted".to_string())?;
            let frame=u32::from_be_bytes(len) as usize;
            if frame==0 || frame>1024*1024+32 { return Err("Invalid transfer frame".into()); }
            let mut enc=vec![0u8;frame];
            stream.read_exact(&mut enc).map_err(|_|"Transfer interrupted".to_string())?;
            let plain=cipher.decrypt(Nonce::from_slice(&chunk_nonce(&base,index)),enc.as_ref()).map_err(|_|"Transfer integrity check failed".to_string())?;
            if received + plain.len() as u64 > size { return Err("Transfer size mismatch".into()); }
            file.write_all(&plain).map_err(|e|e.to_string())?;
            hasher.update(&plain); received+=plain.len() as u64; index=index.wrapping_add(1);
        }
        if received != size { return Err("Transfer size mismatch".into()); }
        file.sync_all().map_err(|e|e.to_string())?;
        let got=format!("{:x}",hasher.finalize());
        if got!=checksum { return Err("Transfer checksum verification failed".into()); }
        Ok(())
    })();
    if let Err(e)=receive_result { let _=std::fs::remove_file(&temp); return Err(e); }
    std::fs::rename(&temp,&final_path).map_err(|e|e.to_string())?;
    let file_size=std::fs::metadata(&final_path).map(|m|m.len() as i64).unwrap_or(size as i64);
    let device_row: Option<i64>=conn.query_row("SELECT id FROM devices WHERE device_id=?1",params![sender],|r|r.get(0)).ok();
    conn.execute("INSERT INTO backups(file_path,file_size,backup_type,status,created_at,checksum,encrypted,database_version,app_version) VALUES(?1,?2,'lan_received','received',datetime('now'),?3,1,'unknown',?4)",params![final_path.to_string_lossy().to_string(),file_size,checksum,header.get("version").and_then(|v|v.as_str()).unwrap_or("unknown")]).map_err(|e|e.to_string())?;
    let received_id: i64=conn.query_row("SELECT id FROM backups WHERE file_path=?1 ORDER BY id DESC LIMIT 1", params![final_path.to_string_lossy().to_string()], |r| r.get(0)).map_err(|e|e.to_string())?;
    if let Err(e)=crate::core::backup::verify_backup(&conn, received_id) {
        let _=conn.execute("DELETE FROM backups WHERE id=?1",params![received_id]);
        let _=std::fs::remove_file(&final_path);
        let _=conn.execute("INSERT INTO transfer_history(device_id,direction,file_name,file_size,status) VALUES(?1,'received',?2,?3,'failed')",params![device_row,safe_name,file_size]);
        return Err(format!("Received backup failed validation: {e}"));
    }
    conn.execute("UPDATE backups SET status='completed' WHERE id=?1",params![received_id]).map_err(|e|e.to_string())?;
    conn.execute("INSERT INTO transfer_history(device_id,direction,file_name,file_size,status) VALUES(?1,'received',?2,?3,'success')",params![device_row,safe_name,file_size]).map_err(|e|e.to_string())?;
    crate::security::audit::log(&conn, "lan_backup_received", Some("backups"), Some(received_id), Some("Encrypted LAN backup received, checksum verified and backup format validated"));
    let ack=serde_json::json!({"success":true,"transfer_id":transfer_id,"message":"Backup received, authenticated and checksum verified."});
    let bytes=serde_json::to_vec(&ack).map_err(|e|e.to_string())?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes()).map_err(|e|e.to_string())?;
    stream.write_all(&bytes).map_err(|e|e.to_string())?;
    Ok(())
}

pub fn send_backup(conn: &Connection, local_id: &str, device_id: &str, backup_id: i64) -> Result<(), String> {
    let (ip,stored): (String,String)=conn.query_row("SELECT COALESCE(ip_address,''),COALESCE(pairing_code,'') FROM devices WHERE device_id=?1 AND status='paired'",params![device_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"Device is not paired or no longer trusted".to_string())?;
    if ip.is_empty(){return Err("Trusted device has no reachable LAN address".into());}
    let secret=decrypt_secret(&stored)?;
    crate::core::backup::verify_backup(conn, backup_id)?;
    let path:String=conn.query_row("SELECT file_path FROM backups WHERE id=?1 AND status='completed'",params![backup_id],|r|r.get(0)).map_err(|_|"Backup not found".to_string())?;
    let meta=std::fs::metadata(&path).map_err(|_|"Backup file is missing".to_string())?; let size=meta.len(); if size>MAX_TRANSFER_SIZE{return Err("Backup exceeds the 512 MB transfer limit".into());}
    let filename=std::path::Path::new(&path).file_name().and_then(|v|v.to_str()).unwrap_or("backup").to_string();
    let data=std::fs::read(&path).map_err(|e|e.to_string())?; let checksum=format!("{:x}",Sha256::digest(&data)); let transfer_id=Uuid::new_v4().to_string(); let nonce=transfer_nonce();
    let auth=transfer_tag(&secret,&transfer_id,local_id,&filename,size,&checksum);
    let header=serde_json::json!({"sender_id":local_id,"filename":filename,"size":size,"sha256":checksum,"transfer_id":transfer_id,"nonce":STANDARD.encode(nonce),"auth":auth,"version":APP_VERSION});
    let address: std::net::SocketAddr = format!("{}:{}", ip, TRANSFER_PORT).parse().map_err(|_|"Invalid trusted device address".to_string())?;
    let mut stream=TcpStream::connect_timeout(&address, TRANSFER_TIMEOUT).map_err(|e|format!("Unable to connect to trusted device: {e}"))?;
    stream.set_write_timeout(Some(TRANSFER_TIMEOUT)).ok();
    stream.set_read_timeout(Some(TRANSFER_TIMEOUT)).ok();
    let hb=serde_json::to_vec(&header).map_err(|e|e.to_string())?; stream.write_all(&(hb.len() as u32).to_be_bytes()).map_err(|e|e.to_string())?; stream.write_all(&hb).map_err(|e|e.to_string())?;
    let cipher=Aes256Gcm::new_from_slice(&secret).map_err(|_|"Unable to initialize transfer encryption".to_string())?; for (i,chunk) in data.chunks(1024*1024).enumerate(){let enc=cipher.encrypt(Nonce::from_slice(&chunk_nonce(&nonce,i as u32)),chunk).map_err(|_|"Unable to encrypt transfer".to_string())?; stream.write_all(&(enc.len() as u32).to_be_bytes()).map_err(|e|e.to_string())?; stream.write_all(&enc).map_err(|e|e.to_string())?;}
    let mut ack_len=[0u8;4];
    if stream.read_exact(&mut ack_len).is_err() {
        let _=conn.execute("INSERT INTO transfer_history(device_id,direction,file_name,file_size,status) SELECT id,'sent',?2,?3,'failed' FROM devices WHERE device_id=?1",params![device_id,filename,size as i64]);
        return Err("Transfer completed locally but the receiving device did not confirm it. Check the receiver's transfer history.".into());
    }
    let ack_len=u32::from_be_bytes(ack_len) as usize; if ack_len==0 || ack_len>4096 { return Err("Invalid transfer confirmation".into()); }
    let mut ack_buf=vec![0u8;ack_len];
    if stream.read_exact(&mut ack_buf).is_err() {
        let _=conn.execute("INSERT INTO transfer_history(device_id,direction,file_name,file_size,status) SELECT id,'sent',?2,?3,'failed' FROM devices WHERE device_id=?1",params![device_id,filename,size as i64]);
        return Err("Receiving device closed the connection before confirming the transfer".into());
    }
    let ack: serde_json::Value=serde_json::from_slice(&ack_buf).map_err(|_|"Invalid transfer confirmation".to_string())?;
    let ok=ack.get("success").and_then(|v|v.as_bool()).unwrap_or(false);
    let ack_id=ack.get("transfer_id").and_then(|v|v.as_str()).unwrap_or("");
    if ack_id!=transfer_id { return Err("Transfer confirmation does not match this transfer".into()); }
    let status=if ok {"success"} else {"failed"};
    let _=conn.execute("INSERT INTO transfer_history(device_id,direction,file_name,file_size,status) SELECT id,'sent',?2,?3,?4 FROM devices WHERE device_id=?1",params![device_id,filename,size as i64,status]);
    if !ok { return Err(ack.get("message").and_then(|v|v.as_str()).unwrap_or("Receiving device rejected the transfer").to_string()); }
    Ok(())
}

pub fn discover(identity: &(String, String)) -> Result<Vec<LanDevice>, String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| format!("LAN discovery failed: {e}"))?;
    socket.set_broadcast(true).map_err(|e| format!("LAN discovery failed: {e}"))?;
    socket.set_read_timeout(Some(Duration::from_millis(650))).map_err(|e| e.to_string())?;
    socket.send_to(b"PAYROLL_DISCOVER_V1", SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT)).map_err(|e| format!("Unable to search the local network: {e}"))?;
    let started = Instant::now(); let mut found = Vec::new(); let mut buffer = [0u8; 1024];
    while started.elapsed() < Duration::from_millis(900) {
        match socket.recv_from(&mut buffer) {
            Ok((size, peer)) => { let ip = match peer.ip() { std::net::IpAddr::V4(v4) => v4, _ => continue }; if let Ok(message) = std::str::from_utf8(&buffer[..size]) { if let Some(device) = decode_hello(message, ip) { if device.device_id != identity.0 && !found.iter().any(|d: &LanDevice| d.device_id == device.device_id) { found.push(device); } } } }
            Err(_) => break,
        }
    }
    found.sort_by(|a,b| a.device_name.to_lowercase().cmp(&b.device_name.to_lowercase())); Ok(found)
}

pub fn create_pairing_request(identity: &(String,String), device: &LanDevice) -> Result<(String,String), String> {
    let code = generate_code(); let nonce = generate_nonce();
    let proof = hash_pair_code(&code, &nonce, &identity.0, &device.device_id);
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| e.to_string())?;
    if device.protocol != MAGIC { return Err("Unsupported LAN protocol".into()); }
    if !compatible_version(&device.app_version) { return Err(format!("Incompatible Payroll System version: {}", device.app_version)); }
    socket.send_to(format!("PAYROLL_PAIR_REQ_V2|{}|{}|{}|{}|{}|{}", identity.0, identity.1.replace('|', " "), APP_VERSION, nonce, proof, now_epoch()).as_bytes(), format!("{}:{}", device.ip_address, DISCOVERY_PORT)).map_err(|e| format!("Unable to request pairing: {e}"))?;
    // Keep the one-time material only until pairing completes or expires.
    // It is replaced with the encrypted shared secret after successful pairing.
    let db_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".payroll-system").join("payroll.db");
    if let Ok(conn) = Connection::open(db_path) {
        let _ = conn.execute("INSERT INTO devices(device_name,device_id,ip_address,pairing_code,status,last_seen,updated_at) VALUES(?1,?2,?3,?4,'pending',datetime('now'),datetime('now')) ON CONFLICT(device_id) DO UPDATE SET device_name=excluded.device_name,ip_address=excluded.ip_address,pairing_code=excluded.pairing_code,status='pending',last_seen=datetime('now'),updated_at=datetime('now')", params![device.device_name, device.device_id, device.ip_address, format!("{}|{}|{}|0|{}", nonce, proof, code, APP_VERSION)]);
    }
    Ok((code, nonce))
}

pub fn list_pairing_requests(conn: &Connection) -> Result<Vec<PairingRequest>, String> {
    let mut stmt = conn.prepare("SELECT device_id,device_name,COALESCE(ip_address,''),COALESCE(pairing_code,''),COALESCE(updated_at,'') FROM devices WHERE status='pending' AND updated_at >= datetime('now','-3 minutes') ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| {
        let raw: String = r.get(3)?;
        let parts: Vec<&str> = raw.split('|').collect();
        Ok(PairingRequest { device_id:r.get(0)?, device_name:r.get(1)?, ip_address:r.get(2)?, app_version:parts.get(4).copied().unwrap_or("unknown").into(), requested_at:r.get(4)? })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn approve_pairing(conn: &Connection, local_id: &str, remote_id: &str, code: &str) -> Result<PairingStatus, String> {
    let raw: String = conn.query_row("SELECT pairing_code FROM devices WHERE device_id=?1 AND status='pending'", params![remote_id], |r| r.get(0)).map_err(|_| "Pairing request not found or expired".to_string())?;
    let p: Vec<&str> = raw.split('|').collect(); if p.len() < 4 { return Err("Invalid pairing request".into()); }
    let nonce=p[0]; let proof=p[1]; let attempts: i64=p[3].parse().unwrap_or(0);
    if p.len() >= 5 && !compatible_version(p[4]) { return Err("This device is running an incompatible Payroll System version".into()); }
    let fresh: i64 = conn.query_row("SELECT CASE WHEN updated_at >= datetime('now','-3 minutes') THEN 1 ELSE 0 END FROM devices WHERE device_id=?1", params![remote_id], |r| r.get(0)).unwrap_or(0);
    if fresh != 1 { return Err("Pairing request expired. Start a new pairing request.".into()); }
    if code.len()!=8 || !code.chars().all(|c| c.is_ascii_digit()) { return Err("Enter the 8-digit pairing code shown on the other device".into()); }
    if hash_pair_code(code, nonce, local_id, remote_id) != proof {
        let next = attempts + 1;
        if next >= MAX_PAIR_ATTEMPTS { let _ = conn.execute("UPDATE devices SET status='revoked',updated_at=datetime('now') WHERE device_id=?1", params![remote_id]); return Err("Too many incorrect pairing attempts. Start a new pairing request.".into()); }
        let updated = format!("{}|{}|{}|{}|{}", nonce, proof, p.get(2).copied().unwrap_or(""), next, p.get(4).copied().unwrap_or(APP_VERSION));
        let _ = conn.execute("UPDATE devices SET pairing_code=?1,updated_at=datetime('now') WHERE device_id=?2", params![updated, remote_id]);
        return Err("Incorrect pairing code".into());
    }
    let secret=derive_secret(code,nonce,local_id,remote_id); let encrypted=encrypt_secret(&secret)?;
    let ip: String = conn.query_row("SELECT COALESCE(ip_address,'') FROM devices WHERE device_id=?1", params![remote_id], |r| r.get(0)).unwrap_or_default();
    conn.execute("UPDATE devices SET pairing_code=?1,status='paired',paired_at=datetime('now'),updated_at=datetime('now') WHERE device_id=?2", params![encrypted,remote_id]).map_err(|e| e.to_string())?;
    if let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) {
        let accept = format!("PAYROLL_PAIR_ACCEPT_V2|{}|{}|{}|{}", local_id, nonce, code, proof);
        let _ = socket.send_to(accept.as_bytes(), format!("{}:{}", ip, DISCOVERY_PORT));
    }
    Ok(PairingStatus { state:"paired".into(), device_id:Some(remote_id.into()), device_name:conn.query_row("SELECT device_name FROM devices WHERE device_id=?1",params![remote_id],|r|r.get(0)).ok(), message:"Device paired securely".into() })
}

pub fn test_connection(conn: &Connection, local_id: &str, device_id: &str) -> Result<LanDevice, String> {
    let (ip, stored, name): (String,String,String) = conn.query_row(
        "SELECT COALESCE(ip_address,''),COALESCE(pairing_code,''),device_name FROM devices WHERE device_id=?1 AND status='paired'",
        params![device_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    ).map_err(|_| "Device is not paired or no longer trusted".to_string())?;
    if ip.is_empty() { return Err("Trusted device has no reachable LAN address".into()); }
    let secret=decrypt_secret(&stored)?;
    let nonce=Uuid::new_v4().to_string();
    let socket=UdpSocket::bind((Ipv4Addr::UNSPECIFIED,0)).map_err(|e|e.to_string())?;
    socket.set_read_timeout(Some(HEALTH_TIMEOUT)).map_err(|e|e.to_string())?;
    let auth=health_tag(&secret,local_id,device_id,&nonce);
    socket.send_to(format!("PAYROLL_HEALTH_V1|{}|{}|{}",local_id,nonce,auth).as_bytes(),format!("{}:{}",ip,DISCOVERY_PORT)).map_err(|e|format!("Unable to contact trusted device: {e}"))?;
    let mut buf=[0u8;512];
    let (n,peer)=socket.recv_from(&mut buf).map_err(|_| "Trusted device did not respond".to_string())?;
    let text=std::str::from_utf8(&buf[..n]).map_err(|_| "Invalid device response".to_string())?;
    let p:Vec<&str>=text.split('|').collect();
    if p.len()!=4 || p[0]!="PAYROLL_HEALTH_OK_V1" || p[1]!=device_id || p[2]!=nonce || p[3]!=health_tag(&secret,device_id,local_id,&nonce) { return Err("Trusted device authentication failed".into()); }
    if peer.ip().to_string()!=ip { return Err("Trusted device address changed; scan again before continuing".into()); }
    conn.execute("UPDATE devices SET last_seen=datetime('now'),updated_at=datetime('now') WHERE device_id=?1",params![device_id]).map_err(|e|e.to_string())?;
    Ok(LanDevice{device_id:device_id.into(),device_name:name,ip_address:ip,app_version:APP_VERSION.into(),protocol:MAGIC.into()})
}

pub fn revoke_device(conn: &Connection, device_id: &str) -> Result<(), String> {
    let changed=conn.execute("UPDATE devices SET status='revoked',pairing_code=NULL,updated_at=datetime('now') WHERE device_id=?1 AND status='paired'",params![device_id]).map_err(|e|e.to_string())?;
    if changed==0 { return Err("Trusted device was not found".into()); }
    Ok(())
}

pub fn list_paired(conn: &Connection) -> Result<Vec<LanDevice>, String> {
    let mut stmt=conn.prepare("SELECT device_id,device_name,COALESCE(ip_address,''),COALESCE(updated_at,'') FROM devices WHERE status='paired' ORDER BY device_name COLLATE NOCASE").map_err(|e|e.to_string())?;
    let rows=stmt.query_map([],|r|Ok(LanDevice{device_id:r.get(0)?,device_name:r.get(1)?,ip_address:r.get(2)?,app_version:"compatible".into(),protocol:MAGIC.into()})).map_err(|e|e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

