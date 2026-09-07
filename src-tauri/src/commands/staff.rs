use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::params;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database::connection::Database;
use crate::security::audit;

#[derive(Serialize, Clone, Debug)]
pub struct Employee {
    pub id: i64,
    pub employee_code: String,
    pub first_name: String,
    pub last_name: String,
    pub gender: Option<String>,
    pub date_of_birth: Option<String>,
    pub national_id: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub department_id: Option<i64>,
    pub department_name: Option<String>,
    pub position_id: Option<i64>,
    pub position_title: Option<String>,
    pub grade: Option<String>,
    pub hire_date: Option<String>,
    pub termination_date: Option<String>,
    pub employment_status: String,
    pub dependants: i64,
    pub rssb_number: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub contract_type_id: Option<i64>,
    pub contract_type_name: Option<String>,
    pub photo_path: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SalaryRecord {
    pub id: i64,
    pub employee_id: i64,
    pub base_salary: f64,
    pub effective_date: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Department {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Position {
    pub id: i64,
    pub title: String,
    pub code: Option<String>,
    pub department_id: i64,
}

#[derive(Deserialize)]
pub struct SaveEmployeeRequest {
    pub id: Option<i64>,
    pub employee_code: String,
    pub first_name: String,
    pub last_name: String,
    pub gender: Option<String>,
    pub date_of_birth: Option<String>,
    pub national_id: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub department_id: Option<i64>,
    pub position_id: Option<i64>,
    pub grade: Option<String>,
    pub hire_date: Option<String>,
    pub employment_status: String,
    pub dependants: Option<i64>,
    pub rssb_number: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub contract_type_id: Option<i64>,
    pub photo_path: Option<String>,
    pub base_salary: Option<f64>,
    pub salary_effective_date: Option<String>,
    pub salary_reason: Option<String>,
}

#[derive(Serialize)]
pub struct StaffResponse {
    pub success: bool,
    pub message: String,
    pub id: Option<i64>,
}

fn row_to_employee(row: &rusqlite::Row) -> rusqlite::Result<Employee> {
    Ok(Employee {
        id: row.get("id")?,
        employee_code: row.get("employee_code")?,
        first_name: row.get("first_name")?,
        last_name: row.get("last_name")?,
        gender: row.get("gender")?,
        date_of_birth: row.get("date_of_birth")?,
        national_id: row.get("national_id")?,
        phone: row.get("phone")?,
        email: row.get("email")?,
        address: row.get("address")?,
        department_id: row.get("department_id")?,
        department_name: row.get("department_name")?,
        position_id: row.get("position_id")?,
        position_title: row.get("position_title")?,
        grade: row.get("grade")?,
        hire_date: row.get("hire_date")?,
        termination_date: row.get("termination_date")?,
        employment_status: row.get("employment_status")?,
        dependants: row.get("dependants")?,
        rssb_number: row.get("rssb_number")?,
        bank_name: row.get("bank_name")?,
        bank_account: row.get("bank_account")?,
        contract_type_id: row.get("contract_type_id")?,
        contract_type_name: row.get("contract_type_name")?,
        photo_path: row.get("photo_path")?,
        is_active: row.get::<_, i64>("is_active")? == 1,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_salary(row: &rusqlite::Row) -> rusqlite::Result<SalaryRecord> {
    Ok(SalaryRecord {
        id: row.get("id")?,
        employee_id: row.get("employee_id")?,
        base_salary: row.get("base_salary")?,
        effective_date: row.get("effective_date")?,
        reason: row.get("reason")?,
        created_at: row.get("created_at")?,
    })
}

const EMPLOYEE_SELECT: &str = r#"
    SELECT e.*, d.name as department_name, p.title as position_title, ct.name as contract_type_name
    FROM employees e
    LEFT JOIN departments d ON e.department_id = d.id
    LEFT JOIN positions p ON e.position_id = p.id
    LEFT JOIN contract_types ct ON e.contract_type_id = ct.id
"#;

#[tauri::command]
pub fn get_employees(db: State<Database>) -> Vec<Employee> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(&format!("{} ORDER BY e.created_at DESC", EMPLOYEE_SELECT)) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], row_to_employee)
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_employee(db: State<Database>, employee_id: i64) -> Option<Employee> {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        &format!("{} WHERE e.id = ?1", EMPLOYEE_SELECT),
        [employee_id],
        row_to_employee,
    )
    .ok()
}

#[tauri::command]
pub fn save_employee(db: State<Database>, request: SaveEmployeeRequest) -> StaffResponse {
    let conn = db.conn.lock().unwrap();

    if request.first_name.trim().is_empty() {
        return StaffResponse { success: false, message: "First name is required".into(), id: None };
    }
    if request.last_name.trim().is_empty() {
        return StaffResponse { success: false, message: "Last name is required".into(), id: None };
    }

    let employee_code = if request.employee_code.trim().is_empty() && request.id.is_none() {
        let prefix = "EMP";
        let mut candidate = String::new();
        for _ in 0..20 {
            let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            candidate = format!("{}-{}", prefix, millis % 100000000);
            let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM employees WHERE employee_code = ?1)", [&candidate], |r| r.get(0)).unwrap_or(true);
            if !exists { break; }
        }
        candidate
    } else {
        request.employee_code.trim().to_string()
    };
    if employee_code.is_empty() {
        return StaffResponse { success: false, message: "Employee code is required".into(), id: None };
    }
    let result = if let Some(id) = request.id {
        conn.execute(
            "UPDATE employees SET
                employee_code = ?1, first_name = ?2, last_name = ?3, gender = ?4,
                date_of_birth = ?5, national_id = ?6, phone = ?7, email = ?8, address = ?9,
                department_id = ?10, position_id = ?11, grade = ?12, hire_date = ?13,
                employment_status = ?14, dependants = ?15, rssb_number = ?16,
                bank_name = ?17, bank_account = ?18, contract_type_id = ?19, photo_path = ?20, updated_at = datetime('now')
             WHERE id = ?21",
            params![
                employee_code, request.first_name, request.last_name, request.gender,
                request.date_of_birth, request.national_id, request.phone, request.email, request.address,
                request.department_id, request.position_id, request.grade, request.hire_date,
                request.employment_status, request.dependants.unwrap_or(0), request.rssb_number,
                request.bank_name, request.bank_account, request.contract_type_id, request.photo_path, id
            ],
        )
        .map(|_| id)
    } else {
        conn.execute(
            "INSERT INTO employees
                (employee_code, first_name, last_name, gender, date_of_birth, national_id,
                 phone, email, address, department_id, position_id, grade, hire_date,
                 employment_status, dependants, rssb_number, bank_name, bank_account, contract_type_id, photo_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                employee_code, request.first_name, request.last_name, request.gender,
                request.date_of_birth, request.national_id, request.phone, request.email, request.address,
                request.department_id, request.position_id, request.grade, request.hire_date,
                request.employment_status, request.dependants.unwrap_or(0), request.rssb_number,
                request.bank_name, request.bank_account, request.contract_type_id, request.photo_path
            ],
        )
        .map(|_| conn.last_insert_rowid())
    };

    let emp_id = match result {
        Ok(id) => id,
        Err(e) => {
            return StaffResponse {
                success: false,
                message: format!("Database error: {}", e),
                id: None,
            };
        }
    };

    // If base_salary provided, insert into salary history (never overwrite)
    if let Some(salary) = request.base_salary {
        if salary > 0.0 {
            let eff_date = request.salary_effective_date.unwrap_or_else(|| {
                conn.query_row("SELECT date('now')", [], |row| row.get::<_, String>(0))
                    .unwrap_or_default()
            });
            let _ = conn.execute(
                "INSERT INTO employee_salary_history (employee_id, base_salary, effective_date, reason)
                 VALUES (?1, ?2, ?3, ?4)",
                params![emp_id, salary, eff_date, request.salary_reason],
            );
        }
    }

    let action = if request.id.is_some() { "employee_updated" } else { "employee_created" };
    audit::log(&conn, action, Some("employees"), Some(emp_id), None);

    StaffResponse {
        success: true,
        message: "Saved".into(),
        id: Some(emp_id),
    }
}

#[tauri::command]
pub fn delete_employee(db: State<Database>, employee_id: i64) -> StaffResponse {
    let conn = db.conn.lock().unwrap();

    let _ = conn.execute(
        "DELETE FROM employee_salary_history WHERE employee_id = ?1",
        [employee_id],
    );
    let _ = conn.execute(
        "DELETE FROM employee_components WHERE employee_id = ?1",
        [employee_id],
    );

    match conn.execute("DELETE FROM employees WHERE id = ?1", [employee_id]) {
        Ok(_) => {
            audit::log(&conn, "employee_deleted", Some("employees"), Some(employee_id), None);
            StaffResponse { success: true, message: "Deleted".into(), id: None }
        }
        Err(e) => StaffResponse { success: false, message: format!("Database error: {}", e), id: None },
    }
}

#[tauri::command]
pub fn get_salary_history(db: State<Database>, employee_id: i64) -> Vec<SalaryRecord> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT * FROM employee_salary_history WHERE employee_id = ?1 ORDER BY effective_date DESC"
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([employee_id], row_to_salary)
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

#[derive(Deserialize)]
pub struct AddSalaryRequest {
    pub employee_id: i64,
    pub base_salary: f64,
    pub effective_date: String,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn add_salary_record(db: State<Database>, request: AddSalaryRequest) -> StaffResponse {
    let conn = db.conn.lock().unwrap();

    if request.base_salary < 0.0 {
        return StaffResponse { success: false, message: "Salary cannot be negative".into(), id: None };
    }
    if request.effective_date.is_empty() {
        return StaffResponse { success: false, message: "Effective date is required".into(), id: None };
    }

    match conn.execute(
        "INSERT INTO employee_salary_history (employee_id, base_salary, effective_date, reason)
         VALUES (?1, ?2, ?3, ?4)",
        params![request.employee_id, request.base_salary, request.effective_date, request.reason],
    ) {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            audit::log(&conn, "salary_added", Some("employee_salary_history"), Some(id), None);
            StaffResponse { success: true, message: "Salary record added".into(), id: Some(id) }
        }
        Err(e) => StaffResponse { success: false, message: format!("Database error: {}", e), id: None },
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct EmployeePayrollOverride {
    pub id: i64,
    pub employee_id: i64,
    pub rule_id: i64,
    pub rule_name: String,
    pub rule_code: String,
    pub override_type: String,
    pub value: Option<f64>,
    pub formula_expression: Option<String>,
    pub base_reference: Option<String>,
    pub effective_date: String,
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct EmployeePayrollOverrideRequest {
    pub id: Option<i64>,
    pub employee_id: i64,
    pub rule_id: i64,
    pub override_type: String,
    pub value: Option<f64>,
    pub formula_expression: Option<String>,
    pub base_reference: Option<String>,
    pub effective_date: String,
    pub is_active: Option<bool>,
}

#[tauri::command]
pub fn get_employee_payroll_overrides(db: State<Database>, employee_id: i64) -> Vec<EmployeePayrollOverride> {
    let conn = db.conn.lock().unwrap();
    let sql = "SELECT o.id,o.employee_id,o.rule_id,r.name,r.code,o.override_type,o.value,o.formula_expression,o.base_reference,o.effective_date,o.is_active
               FROM employee_payroll_overrides o JOIN payroll_rules r ON r.id=o.rule_id
               WHERE o.employee_id=?1 ORDER BY r.sort_order,o.effective_date DESC,o.id DESC";
    let mut stmt = match conn.prepare(sql) { Ok(v) => v, Err(_) => return vec![] };
    stmt.query_map([employee_id], |row| Ok(EmployeePayrollOverride {
        id: row.get(0)?, employee_id: row.get(1)?, rule_id: row.get(2)?, rule_name: row.get(3)?, rule_code: row.get(4)?,
        override_type: row.get(5)?, value: row.get(6)?, formula_expression: row.get(7)?, base_reference: row.get(8)?,
        effective_date: row.get(9)?, is_active: row.get::<_, i64>(10)? == 1,
    })).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn save_employee_payroll_override(db: State<Database>, request: EmployeePayrollOverrideRequest) -> StaffResponse {
    if request.employee_id <= 0 || request.rule_id <= 0 {
        return StaffResponse { success:false, message:"Employee and payroll component are required.".into(), id:None };
    }
    if !matches!(request.override_type.as_str(), "fixed"|"percentage"|"formula") {
        return StaffResponse { success:false, message:"Invalid override type.".into(), id:None };
    }
    if request.effective_date.trim().is_empty() {
        return StaffResponse { success:false, message:"Effective date is required.".into(), id:None };
    }
    let conn = db.conn.lock().unwrap();
    let exists: i64 = conn.query_row("SELECT COUNT(*) FROM employees WHERE id=?1", [request.employee_id], |r| r.get(0)).unwrap_or(0);
    let rule_exists: i64 = conn.query_row("SELECT COUNT(*) FROM payroll_rules WHERE id=?1", [request.rule_id], |r| r.get(0)).unwrap_or(0);
    if exists == 0 || rule_exists == 0 {
        return StaffResponse { success:false, message:"Employee or payroll component was not found.".into(), id:None };
    }
    let result = match request.id {
        Some(id) => conn.execute("UPDATE employee_payroll_overrides SET rule_id=?1,override_type=?2,value=?3,formula_expression=?4,base_reference=?5,effective_date=?6,is_active=?7,updated_at=datetime('now') WHERE id=?8",
            params![request.rule_id,request.override_type,request.value,request.formula_expression,request.base_reference,request.effective_date,request.is_active.unwrap_or(true) as i64,id]).map(|_| id),
        None => conn.execute("INSERT INTO employee_payroll_overrides(employee_id,rule_id,override_type,value,formula_expression,base_reference,effective_date,is_active) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![request.employee_id,request.rule_id,request.override_type,request.value,request.formula_expression,request.base_reference,request.effective_date,request.is_active.unwrap_or(true) as i64]).map(|_| conn.last_insert_rowid()),
    };
    match result {
        Ok(id) => { audit::log(&conn,"employee_payroll_override_saved",Some("employee_payroll_overrides"),Some(id),None); StaffResponse{success:true,message:"Employee payroll component saved.".into(),id:Some(id)} },
        Err(e) => StaffResponse{success:false,message:format!("Database error: {}",e),id:None},
    }
}

#[tauri::command]
pub fn delete_employee_payroll_override(db: State<Database>, id: i64) -> StaffResponse {
    let conn=db.conn.lock().unwrap();
    match conn.execute("DELETE FROM employee_payroll_overrides WHERE id=?1",[id]) {
        Ok(_) => { audit::log(&conn,"employee_payroll_override_deleted",Some("employee_payroll_overrides"),Some(id),None); StaffResponse{success:true,message:"Employee payroll component removed.".into(),id:Some(id)} },
        Err(e) => StaffResponse{success:false,message:format!("Database error: {}",e),id:None},
    }
}

#[tauri::command]
pub fn get_departments(db: State<Database>) -> Vec<Department> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT id, name, code FROM departments WHERE is_active = 1 ORDER BY name"
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| Ok(Department {
        id: row.get(0)?,
        name: row.get(1)?,
        code: row.get(2)?,
    }))
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

#[derive(Serialize, Clone, Debug)]
pub struct Company { pub id: i64, pub name: String, pub legal_name: Option<String>, pub website: Option<String>, pub tin_number: Option<String>, pub rssb_number: Option<String>, pub address: Option<String>, pub phone: Option<String>, pub email: Option<String>, pub tax_id: Option<String>, pub currency: String, pub logo_path: Option<String> }
#[derive(Deserialize)]
pub struct CompanyRequest { pub name: String, pub legal_name: Option<String>, pub website: Option<String>, pub tin_number: Option<String>, pub rssb_number: Option<String>, pub address: Option<String>, pub phone: Option<String>, pub email: Option<String>, pub tax_id: Option<String>, pub currency: String, pub logo_path: Option<String> }
#[derive(Deserialize)]
pub struct DepartmentRequest { pub id: Option<i64>, pub name: String, pub code: Option<String> }
#[derive(Deserialize)]
pub struct PositionRequest { pub id: Option<i64>, pub department_id: i64, pub title: String, pub code: Option<String> }

#[tauri::command]
pub fn get_company(db: State<Database>) -> Option<Company> {
    let conn = db.conn.lock().unwrap();
    conn.query_row("SELECT id,name,legal_name,website,tin_number,rssb_number,address,phone,email,tax_id,currency,logo_path FROM companies WHERE is_active=1 ORDER BY id LIMIT 1",[],|r|Ok(Company{
        id:r.get(0)?, name:r.get(1)?, legal_name:r.get(2)?, website:r.get(3)?, tin_number:r.get(4)?, rssb_number:r.get(5)?,
        address:r.get(6)?, phone:r.get(7)?, email:r.get(8)?, tax_id:r.get(9)?, currency:r.get(10)?, logo_path:r.get(11)?
    })).ok()
}
#[tauri::command]
pub fn save_company(db: State<Database>, request: CompanyRequest) -> StaffResponse {
    let conn=db.conn.lock().unwrap();
    if request.name.trim().is_empty() { return StaffResponse{success:false,message:"Company name is required".into(),id:None}; }
    let existing:Option<i64>=conn.query_row("SELECT id FROM companies WHERE is_active=1 ORDER BY id LIMIT 1",[],|r|r.get(0)).ok();
    let result=match existing {
        Some(id)=>conn.execute("UPDATE companies SET name=?1,legal_name=?2,website=?3,tin_number=?4,rssb_number=?5,address=?6,phone=?7,email=?8,tax_id=?9,currency=?10,logo_path=?11,updated_at=datetime('now') WHERE id=?12",params![request.name,request.legal_name,request.website,request.tin_number,request.rssb_number,request.address,request.phone,request.email,request.tax_id,request.currency,request.logo_path,id]).map(|_|id),
        None=>conn.execute("INSERT INTO companies(name,legal_name,website,tin_number,rssb_number,address,phone,email,tax_id,currency,logo_path) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![request.name,request.legal_name,request.website,request.tin_number,request.rssb_number,request.address,request.phone,request.email,request.tax_id,request.currency,request.logo_path]).map(|_|conn.last_insert_rowid())
    };
    match result{Ok(id)=>StaffResponse{success:true,message:"Company saved".into(),id:Some(id)},Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None}}
}
#[tauri::command]
pub fn save_department(db: State<Database>, request: DepartmentRequest) -> StaffResponse { let conn=db.conn.lock().unwrap(); let company_id:i64=conn.query_row("SELECT id FROM companies WHERE is_active=1 ORDER BY id LIMIT 1",[],|r|r.get(0)).unwrap_or(1); let result=match request.id{Some(id)=>conn.execute("UPDATE departments SET name=?1,code=?2,updated_at=datetime('now') WHERE id=?3",params![request.name,request.code,id]).map(|_|id),None=>conn.execute("INSERT INTO departments(company_id,name,code) VALUES(?1,?2,?3)",params![company_id,request.name,request.code]).map(|_|conn.last_insert_rowid())};match result{Ok(id)=>StaffResponse{success:true,message:"Department saved".into(),id:Some(id)},Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None}} }
#[tauri::command]
pub fn delete_department(db: State<Database>, id:i64)->StaffResponse{let conn=db.conn.lock().unwrap();match conn.execute("UPDATE departments SET is_active=0,updated_at=datetime('now') WHERE id=?1",[id]){Ok(_)=>StaffResponse{success:true,message:"Department deleted".into(),id:Some(id)},Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None}}}
#[tauri::command]
pub fn save_position(db: State<Database>, request: PositionRequest) -> StaffResponse { let conn=db.conn.lock().unwrap(); let result=match request.id{Some(id)=>conn.execute("UPDATE positions SET department_id=?1,title=?2,code=?3,updated_at=datetime('now') WHERE id=?4",params![request.department_id,request.title,request.code,id]).map(|_|id),None=>conn.execute("INSERT INTO positions(department_id,title,code) VALUES(?1,?2,?3)",params![request.department_id,request.title,request.code]).map(|_|conn.last_insert_rowid())};match result{Ok(id)=>StaffResponse{success:true,message:"Position saved".into(),id:Some(id)},Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None}} }
#[tauri::command]
pub fn delete_position(db: State<Database>, id:i64)->StaffResponse{let conn=db.conn.lock().unwrap();match conn.execute("UPDATE positions SET is_active=0,updated_at=datetime('now') WHERE id=?1",[id]){Ok(_)=>StaffResponse{success:true,message:"Position deleted".into(),id:Some(id)},Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None}}}

#[derive(Serialize, Clone, Debug)]
pub struct ContractType {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct ContractTypeRequest { pub id: Option<i64>, pub name: String, pub code: Option<String>, pub description: Option<String> }

#[tauri::command]
pub fn get_contract_types(db: State<Database>) -> Vec<ContractType> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT id,name,code,description,is_active FROM contract_types WHERE is_active=1 ORDER BY name") { Ok(s) => s, Err(_) => return vec![] };
    stmt.query_map([], |r| Ok(ContractType { id:r.get(0)?, name:r.get(1)?, code:r.get(2)?, description:r.get(3)?, is_active:r.get::<_,i64>(4)? == 1 }))
        .ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn save_contract_type(db: State<Database>, request: ContractTypeRequest) -> StaffResponse {
    let conn = db.conn.lock().unwrap();
    if request.name.trim().is_empty() { return StaffResponse { success:false, message:"Contract type name is required".into(), id:None }; }
    let result = match request.id {
        Some(id) => conn.execute("UPDATE contract_types SET name=?1,code=?2,description=?3,updated_at=datetime('now') WHERE id=?4", params![request.name.trim(), request.code, request.description, id]).map(|_| id),
        None => conn.execute("INSERT INTO contract_types(name,code,description) VALUES(?1,?2,?3)", params![request.name.trim(), request.code, request.description]).map(|_| conn.last_insert_rowid()),
    };
    match result { Ok(id) => StaffResponse{success:true,message:"Contract type saved".into(),id:Some(id)}, Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None} }
}

#[tauri::command]
pub fn delete_contract_type(db: State<Database>, id:i64) -> StaffResponse {
    let conn=db.conn.lock().unwrap();
    match conn.execute("UPDATE contract_types SET is_active=0,updated_at=datetime('now') WHERE id=?1", [id]) { Ok(_) => StaffResponse{success:true,message:"Contract type deleted".into(),id:Some(id)}, Err(e)=>StaffResponse{success:false,message:e.to_string(),id:None} }
}

#[tauri::command]
pub fn generate_employee_code(db: State<Database>) -> String {
    let conn = db.conn.lock().unwrap();
    for n in 1..10000_i64 {
        let code = format!("EMP-{n:04}");
        let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM employees WHERE employee_code=?1)", [&code], |r| r.get(0)).unwrap_or(true);
        if !exists { return code; }
    }
    format!("EMP-{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0) % 1_000_000)
}

#[tauri::command]
pub fn get_positions(db: State<Database>, department_id: Option<i64>) -> Vec<Position> {
    let conn = db.conn.lock().unwrap();
    let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match department_id {
        Some(id) => (
            "SELECT id, title, code, department_id FROM positions WHERE is_active = 1 AND department_id = ?1 ORDER BY title".into(),
            vec![Box::new(id)],
        ),
        None => (
            "SELECT id, title, code, department_id FROM positions WHERE is_active = 1 ORDER BY title".into(),
            vec![],
        ),
    };

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    stmt.query_map(param_refs.as_slice(), |row| Ok(Position {
        id: row.get(0)?,
        title: row.get(1)?,
        code: row.get(2)?,
        department_id: row.get(3)?,
    }))
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}
