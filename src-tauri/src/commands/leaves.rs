use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::database::connection::Database;
use crate::security::audit;

#[derive(Serialize)]
pub struct LeaveType { pub id: i64, pub name: String, pub code: String, pub default_days: f64, pub is_paid: bool, pub carry_forward: bool, pub is_active: bool }
#[derive(Serialize)]
pub struct LeaveRecord { pub id: i64, pub employee_id: i64, pub employee_name: String, pub leave_type_id: i64, pub leave_type_name: String, pub start_date: String, pub end_date: String, pub days: f64, pub reason: Option<String>, pub status: String, pub created_at: String }
#[derive(Serialize)]
pub struct LeaveBalance { pub employee_id: i64, pub employee_name: String, pub leave_type_id: i64, pub leave_type_name: String, pub year: i64, pub entitled: f64, pub used: f64, pub remaining: f64 }
#[derive(Serialize)]
pub struct LeaveResponse { pub success: bool, pub message: String, pub id: Option<i64> }
#[derive(Deserialize)]
pub struct LeaveTypeRequest { pub id: Option<i64>, pub name: String, pub code: String, pub default_days: f64, pub is_paid: bool, pub carry_forward: bool }
#[derive(Deserialize)]
pub struct LeaveRequest { pub id: Option<i64>, pub employee_id: i64, pub leave_type_id: i64, pub start_date: String, pub end_date: String, pub days: f64, pub reason: Option<String> }

#[tauri::command]
pub fn get_leave_types(db: State<Database>) -> Vec<LeaveType> {
 let conn=db.conn.lock().unwrap(); let mut s=match conn.prepare("SELECT id,name,code,default_days,is_paid,carry_forward,is_active FROM leave_types ORDER BY name"){Ok(v)=>v,Err(_)=>return vec![]};
 s.query_map([],|r|Ok(LeaveType{id:r.get(0)?,name:r.get(1)?,code:r.get(2)?,default_days:r.get(3)?,is_paid:r.get::<_,i64>(4)?==1,carry_forward:r.get::<_,i64>(5)?==1,is_active:r.get::<_,i64>(6)?==1})).ok().map(|x|x.filter_map(|v|v.ok()).collect()).unwrap_or_default()
}
#[tauri::command]
pub fn save_leave_type(db: State<Database>, request: LeaveTypeRequest) -> LeaveResponse {
 let conn=db.conn.lock().unwrap(); let result=match request.id {Some(id)=>conn.execute("UPDATE leave_types SET name=?1,code=?2,default_days=?3,is_paid=?4,carry_forward=?5,updated_at=datetime('now') WHERE id=?6",params![request.name,request.code,request.default_days,request.is_paid as i64,request.carry_forward as i64,id]).map(|_|id),None=>conn.execute("INSERT INTO leave_types(name,code,default_days,is_paid,carry_forward) VALUES(?1,?2,?3,?4,?5)",params![request.name,request.code,request.default_days,request.is_paid as i64,request.carry_forward as i64]).map(|_|conn.last_insert_rowid())};
 match result {Ok(id)=>LeaveResponse{success:true,message:"Saved".into(),id:Some(id)},Err(e)=>LeaveResponse{success:false,message:e.to_string(),id:None}}
}
#[tauri::command]
pub fn delete_leave_type(db: State<Database>, id:i64)->LeaveResponse{let conn=db.conn.lock().unwrap();match conn.execute("UPDATE leave_types SET is_active=0,updated_at=datetime('now') WHERE id=?1",[id]){Ok(_)=>LeaveResponse{success:true,message:"Deleted".into(),id:Some(id)},Err(e)=>LeaveResponse{success:false,message:e.to_string(),id:None}}}
#[tauri::command]
pub fn get_leave_records(db: State<Database>, year: Option<i64>) -> Vec<LeaveRecord> {
    let conn = db.conn.lock().unwrap();
    // Keep this query to one prepared statement so loading a just-created record
    // behaves exactly the same as loading existing records. LEFT JOIN also keeps
    // the record visible if a related display name is temporarily unavailable.
    let sql = "
        SELECT
            l.id,
            l.employee_id,
            COALESCE(e.first_name || ' ' || e.last_name, 'Unknown employee'),
            l.leave_type_id,
            COALESCE(t.name, 'Unknown leave type'),
            l.start_date,
            l.end_date,
            l.days,
            l.reason,
            l.status,
            l.created_at
        FROM leave_records l
        LEFT JOIN employees e ON e.id = l.employee_id
        LEFT JOIN leave_types t ON t.id = l.leave_type_id
        WHERE (?1 = 0 OR substr(l.start_date, 1, 4) = CAST(?1 AS TEXT))
        ORDER BY l.start_date DESC, l.id DESC
    ";

    let selected_year = year.unwrap_or(0);
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([selected_year], map_record)
        .ok()
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}
fn map_record(r:&rusqlite::Row)->rusqlite::Result<LeaveRecord>{Ok(LeaveRecord{id:r.get(0)?,employee_id:r.get(1)?,employee_name:r.get(2)?,leave_type_id:r.get(3)?,leave_type_name:r.get(4)?,start_date:r.get(5)?,end_date:r.get(6)?,days:r.get(7)?,reason:r.get(8)?,status:r.get(9)?,created_at:r.get(10)?})}
#[tauri::command]
pub fn save_leave_record(db: State<Database>, request: LeaveRequest) -> LeaveResponse { let conn=db.conn.lock().unwrap(); if request.days<=0.0{return LeaveResponse{success:false,message:"Leave days must be greater than zero".into(),id:None}} let result=match request.id{Some(id)=>conn.execute("UPDATE leave_records SET employee_id=?1,leave_type_id=?2,start_date=?3,end_date=?4,days=?5,reason=?6,updated_at=datetime('now') WHERE id=?7",params![request.employee_id,request.leave_type_id,request.start_date,request.end_date,request.days,request.reason,id]).map(|_|id),None=>conn.execute("INSERT INTO leave_records(employee_id,leave_type_id,start_date,end_date,days,reason) VALUES(?1,?2,?3,?4,?5,?6)",params![request.employee_id,request.leave_type_id,request.start_date,request.end_date,request.days,request.reason]).map(|_|conn.last_insert_rowid())}; match result{Ok(id)=>LeaveResponse{success:true,message:"Saved".into(),id:Some(id)},Err(e)=>LeaveResponse{success:false,message:e.to_string(),id:None}} }
#[tauri::command]
pub fn update_leave_status(db: State<Database>, leave_id:i64, status:String)->LeaveResponse{let conn=db.conn.lock().unwrap();match conn.execute("UPDATE leave_records SET status=?1,updated_at=datetime('now') WHERE id=?2",params![status,leave_id]){Ok(_)=>{audit::log(&conn,"leave_status_changed",Some("leave_records"),Some(leave_id),Some(&format!("Leave status changed to {status}")));LeaveResponse{success:true,message:"Status updated".into(),id:Some(leave_id)}},Err(e)=>LeaveResponse{success:false,message:e.to_string(),id:None}}}
#[tauri::command]
pub fn get_leave_balances(db: State<Database>, year:i64)->Vec<LeaveBalance>{let conn=db.conn.lock().unwrap();let mut s=match conn.prepare("SELECT e.id,e.first_name||' '||e.last_name,t.id,t.name,?1,t.default_days,COALESCE(SUM(CASE WHEN l.status='approved' AND substr(l.start_date,1,4)=CAST(?1 AS TEXT) THEN l.days ELSE 0 END),0),t.default_days-COALESCE(SUM(CASE WHEN l.status='approved' AND substr(l.start_date,1,4)=CAST(?1 AS TEXT) THEN l.days ELSE 0 END),0) FROM employees e CROSS JOIN leave_types t LEFT JOIN leave_records l ON l.employee_id=e.id AND l.leave_type_id=t.id WHERE e.is_active=1 AND t.is_active=1 GROUP BY e.id,t.id ORDER BY e.last_name,t.name"){Ok(v)=>v,Err(_)=>return vec![]};s.query_map([year],|r|Ok(LeaveBalance{employee_id:r.get(0)?,employee_name:r.get(1)?,leave_type_id:r.get(2)?,leave_type_name:r.get(3)?,year:r.get(4)?,entitled:r.get(5)?,used:r.get(6)?,remaining:r.get(7)?})).ok().map(|x|x.filter_map(|v|v.ok()).collect()).unwrap_or_default()}
