use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::{params, OptionalExtension};
use rust_decimal::prelude::FromPrimitive;

use crate::database::connection::Database;
use crate::security::audit;
use crate::payroll::engine;
use crate::payroll::models::*;

#[derive(Serialize)]
pub struct RuleListResponse {
    pub rules: Vec<PayrollRule>,
    pub errors: Vec<String>,
}

fn row_to_rule(row: &rusqlite::Row) -> rusqlite::Result<PayrollRule> {
    Ok(PayrollRule {
        id: row.get("id")?,
        name: row.get("name")?,
        code: row.get("code")?,
        component_type: row.get("component_type")?,
        calc_type: row.get("calc_type")?,
        side: row.get("side")?,
        rate: row.get("rate")?,
        formula_expression: row.get("formula_expression")?,
        base_reference: row.get("base_reference")?,
        is_taxable: row.get::<_, i64>("is_taxable")? == 1,
        is_pensionable: row.get::<_, i64>("is_pensionable")? == 1,
        sort_order: row.get("sort_order")?,
        effective_date: row.get("effective_date")?,
        is_active: row.get::<_, i64>("is_active")? == 1,
        version: row.get("version")?,
    })
}

#[tauri::command]
pub fn get_payroll_rules(db: State<Database>) -> RuleListResponse {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT * FROM payroll_rules ORDER BY sort_order ASC"
    ) {
        Ok(s) => s,
        Err(e) => {
            return RuleListResponse {
                rules: vec![],
                errors: vec![format!("DB error: {}", e)],
            };
        }
    };

    let rules: Vec<PayrollRule> = stmt
        .query_map([], row_to_rule)
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut errors = Vec::new();
    if let Err(e) = engine::detect_circular(&rules) {
        errors.push(e);
    }

    RuleListResponse { rules, errors }
}

#[derive(Deserialize)]
pub struct SaveRuleRequest {
    pub id: Option<i64>,
    pub name: String,
    pub code: String,
    pub component_type: String,
    pub calc_type: String,
    pub side: String,
    pub rate: Option<f64>,
    pub formula_expression: Option<String>,
    pub base_reference: Option<String>,
    pub is_taxable: bool,
    pub is_pensionable: bool,
    pub sort_order: i64,
}

#[derive(Serialize)]
pub struct SaveRuleResponse {
    pub success: bool,
    pub message: String,
    pub rule_id: Option<i64>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn save_payroll_rule(db: State<Database>, request: SaveRuleRequest) -> SaveRuleResponse {
    let conn = db.conn.lock().unwrap();

    let existing: Option<PayrollRule> = request.id
        .and_then(|id| {
            conn.query_row(
                "SELECT * FROM payroll_rules WHERE id = ?1",
                [id],
                row_to_rule,
            ).ok()
        });

    let next_version = match &existing {
        Some(r) => r.version + 1,
        None => 1,
    };

    let rate_val = request.rate.map(|r| r.to_string());

    let result = if let Some(id) = request.id {
        conn.execute(
            "UPDATE payroll_rules SET
                name = ?1, code = ?2, component_type = ?3, calc_type = ?4, side = ?5,
                rate = ?6, formula_expression = ?7, base_reference = ?8,
                is_taxable = ?9, is_pensionable = ?10, sort_order = ?11,
                version = ?12, updated_at = datetime('now')
             WHERE id = ?13",
            params![
                request.name, request.code, request.component_type, request.calc_type, request.side,
                rate_val, request.formula_expression, request.base_reference,
                request.is_taxable as i64, request.is_pensionable as i64, request.sort_order,
                next_version, id
            ],
        )
        .map(|_| id)
    } else {
        conn.execute(
            "INSERT INTO payroll_rules
                (name, code, component_type, calc_type, side, rate, formula_expression, base_reference,
                 is_taxable, is_pensionable, sort_order, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                request.name, request.code, request.component_type, request.calc_type, request.side,
                rate_val, request.formula_expression, request.base_reference,
                request.is_taxable as i64, request.is_pensionable as i64, request.sort_order,
                1
            ],
        )
        .map(|_| conn.last_insert_rowid())
    };

    let rule_id = match result {
        Ok(id) => id,
        Err(e) => {
            return SaveRuleResponse {
                success: false,
                message: format!("DB error: {}", e),
                rule_id: None,
                errors: vec![],
            };
        }
    };

    // Save version history
    let _ = conn.execute(
        "INSERT OR REPLACE INTO payroll_rule_versions
            (rule_id, version, name, code, component_type, calc_type, side, rate,
             formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date)
         SELECT id, version, name, code, component_type, calc_type, side, rate,
             formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date
         FROM payroll_rules WHERE id = ?1",
        [rule_id]
    );

    // Update dependencies
    let _ = conn.execute(
        "DELETE FROM payroll_rule_dependencies WHERE rule_id = ?1",
        [rule_id]
    );

    let deps = extract_formula_deps(&request);
    for dep in &deps {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code) VALUES (?1, ?2)",
            params![rule_id, dep]
        );
    }

    // Validate circular
    let all_rules: Vec<PayrollRule> = match conn.prepare("SELECT * FROM payroll_rules ORDER BY sort_order ASC") {
        Ok(mut stmt) => match stmt.query_map([], row_to_rule) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let mut errors = Vec::new();
    if let Err(e) = engine::detect_circular(&all_rules) {
        errors.push(e);
    }

    SaveRuleResponse {
        success: errors.is_empty(),
        message: if errors.is_empty() { "Saved".to_string() } else { "Saved with warnings".to_string() },
        rule_id: Some(rule_id),
        errors,
    }
}

fn extract_formula_deps(req: &SaveRuleRequest) -> Vec<String> {
    let mut deps = Vec::new();
    if req.calc_type == "formula" {
        if let Some(expr) = &req.formula_expression {
            let mut current = String::new();
            for ch in expr.chars() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    current.push(ch);
                } else if !current.is_empty() {
                    if current.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                        && !deps.contains(&current)
                        && !is_builtin(&current)
                    {
                        deps.push(current.clone());
                    }
                    current.clear();
                }
            }
            if !current.is_empty()
                && current.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && !deps.contains(&current)
                && !is_builtin(&current)
            {
                deps.push(current);
            }
        }
    }
    if let Some(base) = &req.base_reference {
        if !deps.contains(base) {
            deps.push(base.clone());
        }
    }
    deps
}

fn is_builtin(id: &str) -> bool {
    matches!(id, "true" | "false" | "min" | "max" | "round" | "floor" | "ceil" | "abs")
}

#[tauri::command]
pub fn toggle_payroll_rule(db: State<Database>, rule_id: i64, is_active: bool) -> SaveRuleResponse {
    let conn = db.conn.lock().unwrap();
    match conn.execute(
        "UPDATE payroll_rules SET is_active = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![is_active as i64, rule_id],
    ) {
        Ok(_) => SaveRuleResponse {
            success: true,
            message: "Updated".to_string(),
            rule_id: Some(rule_id),
            errors: vec![],
        },
        Err(e) => SaveRuleResponse {
            success: false,
            message: format!("DB error: {}", e),
            rule_id: None,
            errors: vec![],
        },
    }
}

#[tauri::command]
pub fn delete_payroll_rule(db: State<Database>, rule_id: i64) -> SaveRuleResponse {
    let conn = db.conn.lock().unwrap();
    let _ = conn.execute("DELETE FROM payroll_rule_dependencies WHERE rule_id = ?1", [rule_id]);
    let _ = conn.execute("DELETE FROM payroll_rule_versions WHERE rule_id = ?1", [rule_id]);
    match conn.execute("DELETE FROM payroll_rules WHERE id = ?1", [rule_id]) {
        Ok(_) => SaveRuleResponse {
            success: true,
            message: "Deleted".to_string(),
            rule_id: None,
            errors: vec![],
        },
        Err(e) => SaveRuleResponse {
            success: false,
            message: format!("DB error: {}", e),
            rule_id: None,
            errors: vec![],
        },
    }
}

#[derive(Serialize)]
pub struct VersionHistory {
    pub versions: Vec<RuleVersion>,
}

#[derive(Serialize)]
pub struct RuleVersion {
    pub id: i64,
    pub rule_id: i64,
    pub version: i64,
    pub name: String,
    pub code: String,
    pub component_type: String,
    pub calc_type: String,
    pub side: String,
    pub rate: Option<f64>,
    pub formula_expression: Option<String>,
    pub base_reference: Option<String>,
    pub is_taxable: bool,
    pub is_pensionable: bool,
    pub sort_order: i64,
    pub effective_date: String,
    pub created_at: String,
}

#[tauri::command]
pub fn get_rule_versions(db: State<Database>, rule_id: i64) -> VersionHistory {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT * FROM payroll_rule_versions WHERE rule_id = ?1 ORDER BY version DESC"
    ) {
        Ok(s) => s,
        Err(_) => return VersionHistory { versions: vec![] },
    };

    let versions = stmt.query_map([rule_id], |row| {
        Ok(RuleVersion {
            id: row.get("id")?,
            rule_id: row.get("rule_id")?,
            version: row.get("version")?,
            name: row.get("name")?,
            code: row.get("code")?,
            component_type: row.get("component_type")?,
            calc_type: row.get("calc_type")?,
            side: row.get("side")?,
            rate: row.get("rate")?,
            formula_expression: row.get("formula_expression")?,
            base_reference: row.get("base_reference")?,
            is_taxable: row.get::<_, i64>("is_taxable")? == 1,
            is_pensionable: row.get::<_, i64>("is_pensionable")? == 1,
            sort_order: row.get("sort_order")?,
            effective_date: row.get("effective_date")?,
            created_at: row.get("created_at")?,
        })
    }).ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    VersionHistory { versions }
}

#[derive(Deserialize)]
pub struct TestFormulaRequest {
    pub expression: String,
    pub inputs: Vec<TestInput>,
}

#[tauri::command]
pub fn test_formula(db: State<Database>, request: TestFormulaRequest) -> FormulaTestResult {
    let conn = db.conn.lock().unwrap();

    let rules: Vec<PayrollRule> = match conn.prepare("SELECT * FROM payroll_rules WHERE is_active = 1 ORDER BY sort_order ASC") {
        Ok(mut stmt) => match stmt.query_map([], row_to_rule) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    engine::test_formula(&request.expression, &request.inputs, &rules)
}

#[derive(Deserialize)]
pub struct PreviewCalcRequest {
    pub basic_salary: f64,
}

#[tauri::command]
pub fn preview_calculation(db: State<Database>, request: PreviewCalcRequest) -> CalcResult {
    let conn = db.conn.lock().unwrap();

    let rules: Vec<PayrollRule> = match conn.prepare("SELECT * FROM payroll_rules WHERE is_active = 1 ORDER BY sort_order ASC") {
        Ok(mut stmt) => match stmt.query_map([], row_to_rule) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let mut inputs = std::collections::HashMap::new();
    inputs.insert(
        "BASIC".to_string(),
        rust_decimal::Decimal::from_f64(request.basic_salary).unwrap_or(rust_decimal_macros::dec!(0)),
    );

    engine::calculate(&rules, &inputs)
}

#[tauri::command]
pub fn validate_rules(db: State<Database>) -> Vec<String> {
    let conn = db.conn.lock().unwrap();

    let rules: Vec<PayrollRule> = match conn.prepare("SELECT * FROM payroll_rules WHERE is_active = 1 ORDER BY sort_order ASC") {
        Ok(mut stmt) => match stmt.query_map([], row_to_rule) {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let mut errors = Vec::new();
    if let Err(e) = engine::detect_circular(&rules) {
        errors.push(e);
    }

    for rule in &rules {
        if rule.calc_type == "formula" {
            if rule.formula_expression.is_none() || rule.formula_expression.as_ref().unwrap().is_empty() {
                errors.push(format!("Rule '{}' has formula type but no expression", rule.code));
            }
        }
        if rule.calc_type == "percentage" && rule.rate.is_none() {
            errors.push(format!("Rule '{}' has percentage type but no rate", rule.code));
        }
    }

    errors
}

#[derive(Serialize, Clone, Debug)]
pub struct PayrollPeriodRow {
    pub id: i64,
    pub period_name: String,
    pub start_date: String,
    pub end_date: String,
    pub pay_date: Option<String>,
    pub status: String,
    pub config_version: i64,
    pub record_count: i64,
    pub total_gross: f64,
    pub total_deductions: f64,
    pub total_tax: f64,
    pub total_net: f64,
    pub employer_contributions: f64,
}

#[derive(Deserialize)]
pub struct PayrollPeriodRequest {
    pub id: Option<i64>,
    pub period_name: String,
    pub start_date: String,
    pub end_date: String,
    pub pay_date: Option<String>,
}

#[derive(Serialize)]
pub struct PayrollPeriodResponse {
    pub success: bool,
    pub message: String,
    pub id: Option<i64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct PayrollRecordRow {
    pub id: i64,
    pub employee_id: i64,
    pub employee_code: String,
    pub employee_name: String,
    pub department_name: Option<String>,
    pub position_title: Option<String>,
    pub base_salary: f64,
    pub gross_earnings: f64,
    pub total_deductions: f64,
    pub total_tax: f64,
    pub net_pay: f64,
    pub employer_contributions: f64,
    pub status: String,
    pub config_version: i64,
    pub calculation_snapshot: Option<String>,
}

#[derive(Serialize)]
pub struct PayrollRunResponse {
    pub success: bool,
    pub message: String,
    pub period_id: i64,
    pub records: Vec<PayrollRecordRow>,
    pub errors: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct LoanRow {
    pub id: i64,
    pub employee_id: i64,
    pub employee_name: String,
    pub principal: f64,
    pub interest_rate: f64,
    pub total_amount: f64,
    pub installment_amount: f64,
    pub total_installments: i64,
    pub paid_installments: i64,
    pub remaining_amount: f64,
    pub start_date: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct LoanRequest {
    pub id: Option<i64>,
    pub employee_id: i64,
    pub principal: f64,
    pub interest_rate: f64,
    pub total_amount: f64,
    pub installment_amount: f64,
    pub total_installments: i64,
    pub start_date: String,
}

#[derive(Serialize)]
pub struct PayslipRow {
    pub id: i64,
    pub payroll_record_id: i64,
    pub employee_id: i64,
    pub employee_name: String,
    pub employee_code: String,
    pub period_name: String,
    pub payslip_number: String,
    pub net_pay: f64,
    pub generated_at: String,
    pub calculation_snapshot: Option<String>,
}

fn current_config_version(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(version),1) FROM payroll_rule_versions",
        [],
        |row| row.get(0),
    ).unwrap_or(1)
}

#[tauri::command]
pub fn get_payroll_periods(db: State<Database>) -> Vec<PayrollPeriodRow> {
    let conn = db.conn.lock().unwrap();
    let sql = r#"
        SELECT p.id, p.period_name, p.start_date, p.end_date, p.pay_date, p.status, p.config_version,
               COUNT(DISTINCT r.employee_id), COALESCE(SUM(r.gross_earnings),0), COALESCE(SUM(r.total_deductions),0),
               COALESCE(SUM(r.total_tax),0), COALESCE(SUM(r.net_pay),0),
               COALESCE(SUM(json_extract(r.calculation_snapshot, '$.totals.employer_contributions')),0)
        FROM payroll_periods p
        LEFT JOIN payroll_records r ON r.period_id = p.id
        GROUP BY p.id
        ORDER BY p.start_date DESC, p.id DESC
    "#;
    let mut stmt = match conn.prepare(sql) { Ok(v) => v, Err(_) => return vec![] };
    stmt.query_map([], |r| Ok(PayrollPeriodRow {
        id: r.get(0)?, period_name: r.get(1)?, start_date: r.get(2)?, end_date: r.get(3)?, pay_date: r.get(4)?, status: r.get(5)?, config_version: r.get(6)?, record_count: r.get(7)?, total_gross: r.get(8)?, total_deductions: r.get(9)?, total_tax: r.get(10)?, total_net: r.get(11)?, employer_contributions: r.get(12)?,
    })).ok().map(|rows| rows.filter_map(|v| v.ok()).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn save_payroll_period(db: State<Database>, request: PayrollPeriodRequest) -> PayrollPeriodResponse {
    if request.period_name.trim().is_empty() || request.start_date > request.end_date {
        return PayrollPeriodResponse { success: false, message: "Enter a valid payroll period and date range.".into(), id: None };
    }
    let conn = db.conn.lock().unwrap();
    let overlap: i64 = conn.query_row(
        "SELECT COUNT(*) FROM payroll_periods WHERE status IN ('open','processing') AND start_date <= ?1 AND end_date >= ?2 AND (?3 IS NULL OR id != ?3)",
        params![request.end_date, request.start_date, request.id],
        |r| r.get(0),
    ).unwrap_or(0);
    if overlap > 0 {
        return PayrollPeriodResponse { success:false, message:"This payroll period overlaps another open payroll period.".into(), id:request.id };
    }
    let config_version = current_config_version(&conn);
    let result = match request.id {
        Some(id) => conn.execute(
            "UPDATE payroll_periods SET period_name=?1,start_date=?2,end_date=?3,pay_date=?4,updated_at=datetime('now') WHERE id=?5 AND status='open'",
            params![request.period_name, request.start_date, request.end_date, request.pay_date, id],
        ).map(|_| id),
        None => conn.execute(
            "INSERT INTO payroll_periods(period_name,start_date,end_date,pay_date,status,config_version) VALUES(?1,?2,?3,?4,'open',?5)",
            params![request.period_name, request.start_date, request.end_date, request.pay_date, config_version],
        ).map(|_| conn.last_insert_rowid()),
    };
    match result {
        Ok(id) => { audit::log(&conn, if request.id.is_some() { "payroll_period_updated" } else { "payroll_period_created" }, Some("payroll_periods"), Some(id), Some("Payroll period saved")); PayrollPeriodResponse { success: true, message: "Payroll period saved.".into(), id: Some(id) } },
        Err(e) => PayrollPeriodResponse { success: false, message: e.to_string(), id: None },
    }
}

#[tauri::command]
pub fn delete_payroll_period(db: State<Database>, period_id: i64) -> PayrollPeriodResponse {
    let conn = db.conn.lock().unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM payroll_records WHERE period_id=?1", [period_id], |r| r.get(0)).unwrap_or(0);
    if count > 0 { return PayrollPeriodResponse { success:false, message:"A processed payroll cannot be deleted. Reopen only through an explicit recovery workflow.".into(), id:Some(period_id) }; }
    match conn.execute("DELETE FROM payroll_periods WHERE id=?1 AND status='open'", [period_id]) {
        Ok(_) => PayrollPeriodResponse { success:true, message:"Payroll period deleted.".into(), id:Some(period_id) },
        Err(e) => PayrollPeriodResponse { success:false, message:e.to_string(), id:None },
    }
}

#[tauri::command]
pub fn delete_payroll_history(db: State<Database>, period_id: i64) -> PayrollPeriodResponse {
    let mut conn = db.conn.lock().unwrap();
    let status: String = match conn.query_row("SELECT status FROM payroll_periods WHERE id=?1", [period_id], |r| r.get(0)) {
        Ok(v) => v,
        Err(_) => return PayrollPeriodResponse { success:false, message:"Payroll history was not found.".into(), id:Some(period_id) },
    };
    if status != "closed" && status != "locked" {
        return PayrollPeriodResponse { success:false, message:"Only finalized payroll history can be deleted here.".into(), id:Some(period_id) };
    }
    let tx = match conn.transaction() {
        Ok(v) => v,
        Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(period_id) },
    };
    let loan_ids: Vec<i64> = match tx.prepare("SELECT DISTINCT loan_id FROM loan_payments WHERE payroll_record_id IN (SELECT id FROM payroll_records WHERE period_id=?1)") {
        Ok(mut stmt) => stmt.query_map([period_id], |r| r.get(0)).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default(),
        Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(period_id) },
    };
    let result: Result<(), rusqlite::Error> = (|| {
        tx.execute("DELETE FROM payslips WHERE period_id=?1", [period_id])?;
        tx.execute("DELETE FROM loan_payments WHERE payroll_record_id IN (SELECT id FROM payroll_records WHERE period_id=?1)", [period_id])?;
        tx.execute("DELETE FROM payroll_items WHERE payroll_record_id IN (SELECT id FROM payroll_records WHERE period_id=?1)", [period_id])?;
        tx.execute("DELETE FROM payroll_records WHERE period_id=?1", [period_id])?;
        tx.execute("DELETE FROM payroll_periods WHERE id=?1 AND status IN ('closed','locked')", [period_id])?;
        for loan_id in &loan_ids {
            tx.execute("UPDATE loans SET paid_installments=(SELECT COUNT(*) FROM loan_payments WHERE loan_id=?1), status=CASE WHEN status='cancelled' THEN 'cancelled' WHEN (SELECT COUNT(*) FROM loan_payments WHERE loan_id=?1)>=total_installments THEN 'paid' ELSE 'active' END, updated_at=datetime('now') WHERE id=?1", [loan_id])?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        return PayrollPeriodResponse { success:false, message:format!("Unable to delete payroll history: {e}"), id:Some(period_id) };
    }
    audit::log(&tx, "payroll_history_deleted", Some("payroll_periods"), Some(period_id), Some("Finalized payroll history, frozen records, payslips and linked loan payments deleted"));
    if let Err(e) = tx.commit() {
        return PayrollPeriodResponse { success:false, message:format!("Unable to commit payroll history deletion: {e}"), id:Some(period_id) };
    }
    PayrollPeriodResponse { success:true, message:"Payroll history deleted and linked loan balances restored.".into(), id:Some(period_id) }
}

#[tauri::command]
pub fn clear_payroll_history(db: State<Database>) -> PayrollPeriodResponse {
    let mut conn = db.conn.lock().unwrap();
    let tx = match conn.transaction() {
        Ok(v) => v,
        Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:None },
    };
    let loan_ids: Vec<i64> = match tx.prepare("SELECT DISTINCT loan_id FROM loan_payments WHERE payroll_record_id IN (SELECT r.id FROM payroll_records r JOIN payroll_periods p ON p.id=r.period_id WHERE p.status IN ('closed','locked'))") {
        Ok(mut stmt) => stmt.query_map([], |r| r.get(0)).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default(),
        Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:None },
    };
    let period_count: i64 = tx.query_row("SELECT COUNT(*) FROM payroll_periods WHERE status IN ('closed','locked')", [], |r| r.get(0)).unwrap_or(0);
    let result: Result<i64, rusqlite::Error> = (|| {
        tx.execute("DELETE FROM payslips WHERE period_id IN (SELECT id FROM payroll_periods WHERE status IN ('closed','locked'))", [])?;
        tx.execute("DELETE FROM loan_payments WHERE payroll_record_id IN (SELECT r.id FROM payroll_records r JOIN payroll_periods p ON p.id=r.period_id WHERE p.status IN ('closed','locked'))", [])?;
        tx.execute("DELETE FROM payroll_items WHERE payroll_record_id IN (SELECT r.id FROM payroll_records r JOIN payroll_periods p ON p.id=r.period_id WHERE p.status IN ('closed','locked'))", [])?;
        tx.execute("DELETE FROM payroll_records WHERE period_id IN (SELECT id FROM payroll_periods WHERE status IN ('closed','locked'))", [])?;
        tx.execute("DELETE FROM payroll_periods WHERE status IN ('closed','locked')", [])?;
        for loan_id in &loan_ids {
            tx.execute("UPDATE loans SET paid_installments=(SELECT COUNT(*) FROM loan_payments WHERE loan_id=?1), status=CASE WHEN status='cancelled' THEN 'cancelled' WHEN (SELECT COUNT(*) FROM loan_payments WHERE loan_id=?1)>=total_installments THEN 'paid' ELSE 'active' END, updated_at=datetime('now') WHERE id=?1", [loan_id])?;
        }
        Ok(period_count)
    })();
    let deleted = match result { Ok(v) => v, Err(e) => return PayrollPeriodResponse { success:false, message:format!("Unable to clear payroll history: {e}"), id:None } };
    audit::log(&tx, "payroll_history_cleared", Some("payroll_periods"), None, Some(&format!("Finalized payroll history cleared; {deleted} periods removed")));
    if let Err(e) = tx.commit() { return PayrollPeriodResponse { success:false, message:format!("Unable to commit payroll history deletion: {e}"), id:None }; }
    PayrollPeriodResponse { success:true, message:format!("All finalized payroll history cleared: {deleted} periods removed."), id:None }
}

fn active_rules(conn: &rusqlite::Connection) -> Result<Vec<PayrollRule>, String> {
    let mut stmt = conn.prepare("SELECT * FROM payroll_rules WHERE is_active=1 ORDER BY sort_order ASC").map_err(|e| e.to_string())?;
    let rules: Vec<PayrollRule> = stmt.query_map([], row_to_rule).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
    if let Err(e) = engine::detect_circular(&rules) { return Err(e); }
    Ok(rules)
}

fn salary_for_employee(conn: &rusqlite::Connection, employee_id: i64, end_date: &str) -> Result<f64, String> {
    conn.query_row(
        "SELECT base_salary FROM employee_salary_history WHERE employee_id=?1 AND effective_date<=?2 ORDER BY effective_date DESC, id DESC LIMIT 1",
        params![employee_id, end_date],
        |r| r.get(0),
    ).optional().map_err(|e| e.to_string()).map(|v| v.unwrap_or(0.0))
}

fn due_loan_amount(conn: &rusqlite::Connection, employee_id: i64) -> f64 {
    let value: f64 = conn.query_row(
        "SELECT COALESCE(SUM(installment_amount),0) FROM loans WHERE employee_id=?1 AND status='active' AND paid_installments<total_installments",
        [employee_id], |r| r.get(0)).unwrap_or(0.0);
    value
}

fn date_ordinal(date: &str) -> Result<i64, String> {
    let parts: Vec<i64> = date.split('-').map(|v| v.parse::<i64>().map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    if parts.len() != 3 { return Err(format!("Invalid date: {}", date)); }
    let mut y = parts[0];
    let m = parts[1];
    let d = parts[2];
    if !(1..=12).contains(&m) || d < 1 || d > 31 { return Err(format!("Invalid date: {}", date)); }
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146097 + doe)
}

fn date_range_days(start: &str, end: &str) -> Result<i64, String> {
    let a = date_ordinal(start)?;
    let b = date_ordinal(end)?;
    if b < a { return Ok(0); }
    Ok(b - a + 1)
}

fn scheduled_days(start: &str, end: &str) -> Result<i64, String> {
    let a = date_ordinal(start)?;
    let b = date_ordinal(end)?;
    if b < a { return Ok(0); }
    let mut count = 0;
    for offset in 0..=(b - a) {
        // 1970-01-01 was Thursday (4 when Sunday=0).
        let weekday = (4 + (a - 719468) + offset).rem_euclid(7);
        if weekday != 0 { count += 1; }
    }
    Ok(count)
}

fn employee_paid_days(conn: &rusqlite::Connection, employee_id: i64, period_start: &str, period_end: &str) -> Result<(i64, i64, f64), String> {
    let (hire_date, termination_date): (Option<String>, Option<String>) = conn.query_row(
        "SELECT hire_date, termination_date FROM employees WHERE id=?1", [employee_id], |r| Ok((r.get(0)?, r.get(1)?))
    ).map_err(|e| e.to_string())?;

    let effective_start = hire_date.as_deref().filter(|d| *d > period_start).unwrap_or(period_start);
    let effective_end = termination_date.as_deref().filter(|d| *d < period_end).unwrap_or(period_end);
    let scheduled = scheduled_days(effective_start, effective_end)?;
    if scheduled <= 0 { return Ok((0, 0, 0.0)); }

    let mut unpaid_leave_days = 0.0;
    let mut stmt = conn.prepare(
        "SELECT l.start_date,l.end_date,l.days,t.is_paid FROM leave_records l JOIN leave_types t ON t.id=l.leave_type_id WHERE l.employee_id=?1 AND l.status='approved' AND l.end_date>=?2 AND l.start_date<=?3"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![employee_id, effective_start, effective_end], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?, r.get::<_, i64>(3)? == 1))
    }).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        let (leave_start, leave_end, leave_days, is_paid) = row;
        if is_paid || leave_days <= 0.0 { continue; }
        let overlap_start = if leave_start.as_str() > effective_start { leave_start.clone() } else { effective_start.to_string() };
        let overlap_end = if leave_end.as_str() < effective_end { leave_end.clone() } else { effective_end.to_string() };
        let overlap_scheduled = scheduled_days(&overlap_start, &overlap_end)?;
        if overlap_scheduled <= 0 { continue; }
        let full_scheduled = scheduled_days(&leave_start, &leave_end)?;
        let proportional = if full_scheduled > 0 { leave_days * overlap_scheduled as f64 / full_scheduled as f64 } else { 0.0 };
        unpaid_leave_days += proportional.min(overlap_scheduled as f64);
    }

    let paid = ((scheduled as f64) - unpaid_leave_days).max(0.0);
    Ok((paid.round() as i64, scheduled, unpaid_leave_days))
}

fn calculate_employee(conn: &rusqlite::Connection, employee_id: i64, period_start: &str, period_end: &str, rules: &[PayrollRule]) -> Result<CalcResult, String> {
    let basic = salary_for_employee(conn, employee_id, period_end)?;
    let loan_due = due_loan_amount(conn, employee_id);
    let mut inputs = std::collections::HashMap::new();
    inputs.insert("BASIC".to_string(), rust_decimal::Decimal::from_f64(basic).unwrap_or(rust_decimal_macros::dec!(0)));
    inputs.insert("LOAN_DED".to_string(), rust_decimal::Decimal::from_f64(loan_due).unwrap_or(rust_decimal_macros::dec!(0)));

    let mut effective_rules = rules.to_vec();
    let mut stmt = conn.prepare("SELECT rule_id, override_type, value, formula_expression, base_reference FROM employee_payroll_overrides WHERE employee_id=?1 AND is_active=1 AND effective_date<=?2 ORDER BY effective_date DESC, id DESC").map_err(|e| e.to_string())?;
    let overrides = stmt.query_map(params![employee_id, period_end], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<f64>>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, Option<String>>(4)?))
    }).map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    for row in overrides.flatten() {
        let (rule_id, override_type, value, formula_expression, base_reference) = row;
        if !seen.insert(rule_id) { continue; }
        if let Some(rule) = effective_rules.iter_mut().find(|r| r.id == rule_id) {
            match override_type.as_str() {
                "fixed" => { rule.calc_type = "fixed".into(); rule.rate = value; rule.formula_expression = None; },
                "percentage" => { rule.calc_type = "percentage".into(); rule.rate = value; rule.base_reference = base_reference.or_else(|| rule.base_reference.clone()); rule.formula_expression = None; },
                "formula" => { rule.calc_type = "formula".into(); rule.formula_expression = formula_expression; rule.base_reference = base_reference.or_else(|| rule.base_reference.clone()); },
                _ => {}
            }
        }
    }
    let mut result = engine::calculate(&effective_rules, &inputs);
    if result.errors.is_empty() {
        let (paid_days, scheduled_days, unpaid_leave_days) = employee_paid_days(conn, employee_id, period_start, period_end)?;
        // The snapshot carries the authoritative attendance/payable-day figures used by the payslip.
        // Payroll amounts remain rule-driven; this metadata only describes why the employee was paid for the period.
        result.errors.clear();
        result.items.shrink_to_fit();
        // Encode attendance metadata through a dedicated synthetic item only for snapshot transport.
        // It is filtered out of monetary reports by the UI.
        result.items.push(CalcItem {
            code: "__PAID_DAYS".into(), name: "Paid days".into(), component_type: "metadata".into(), calc_type: "fixed".into(), side: "employee".into(), rate: None, formula: None, base_reference: None, amount: paid_days.to_string(),
        });
        result.items.push(CalcItem {
            code: "__SCHEDULED_DAYS".into(), name: "Scheduled payable days".into(), component_type: "metadata".into(), calc_type: "fixed".into(), side: "employee".into(), rate: None, formula: None, base_reference: None, amount: scheduled_days.to_string(),
        });
        result.items.push(CalcItem {
            code: "__UNPAID_LEAVE_DAYS".into(), name: "Unpaid leave days".into(), component_type: "metadata".into(), calc_type: "fixed".into(), side: "employee".into(), rate: None, formula: None, base_reference: None, amount: unpaid_leave_days.to_string(),
        });
        Ok(result)
    } else { Err(result.errors.join("; ")) }
}

fn save_calculation_record(conn: &rusqlite::Connection, period_id: i64, employee_id: i64, config_version: i64, result: &CalcResult, status: &str) -> Result<i64, String> {
    let base_salary = result.items.iter().find(|i| i.code == "BASIC").and_then(|i| i.amount.parse::<f64>().ok()).unwrap_or(0.0);
    let employer = result.employer_contributions.parse::<f64>().unwrap_or(0.0);
    let paid_days = result.items.iter().find(|i| i.code == "__PAID_DAYS").and_then(|i| i.amount.parse::<i64>().ok()).unwrap_or(0);
    let scheduled_days = result.items.iter().find(|i| i.code == "__SCHEDULED_DAYS").and_then(|i| i.amount.parse::<i64>().ok()).unwrap_or(0);
    let unpaid_leave_days = result.items.iter().find(|i| i.code == "__UNPAID_LEAVE_DAYS").and_then(|i| i.amount.parse::<f64>().ok()).unwrap_or(0.0);
    let monetary_items: Vec<&CalcItem> = result.items.iter().filter(|i| i.component_type != "metadata").collect();
    let snapshot = serde_json::json!({
        "inputs": { "basic_salary": base_salary },
        "attendance": { "paid_days": paid_days, "scheduled_days": scheduled_days, "unpaid_leave_days": unpaid_leave_days },
        "items": &monetary_items,
        "totals": {
            "gross_earnings": result.gross_earnings,
            "total_deductions": result.total_deductions,
            "total_tax": result.total_tax,
            "net_pay": result.net_pay,
            "employer_contributions": employer,
        },
        "config_version": config_version,
        "created_at": chrono_like_now(),
    }).to_string();

    let existing: Option<i64> = conn.query_row("SELECT id FROM payroll_records WHERE period_id=?1 AND employee_id=?2 ORDER BY id DESC LIMIT 1", params![period_id, employee_id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
    let id = if let Some(id) = existing {
        conn.execute("UPDATE payroll_records SET base_salary=?1,gross_earnings=?2,total_deductions=?3,total_tax=?4,net_pay=?5,calculation_snapshot=?6,status=?7,config_version=?8,updated_at=datetime('now') WHERE id=?9",
            params![base_salary,result.gross_earnings.parse::<f64>().unwrap_or(0.0),result.total_deductions.parse::<f64>().unwrap_or(0.0),result.total_tax.parse::<f64>().unwrap_or(0.0),result.net_pay.parse::<f64>().unwrap_or(0.0),snapshot,status,config_version,id]).map_err(|e|e.to_string())?;
        conn.execute("DELETE FROM payroll_items WHERE payroll_record_id=?1", [id]).ok();
        id
    } else {
        conn.execute("INSERT INTO payroll_records(period_id,employee_id,base_salary,gross_earnings,total_deductions,total_tax,net_pay,calculation_snapshot,status,config_version) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![period_id,employee_id,base_salary,result.gross_earnings.parse::<f64>().unwrap_or(0.0),result.total_deductions.parse::<f64>().unwrap_or(0.0),result.total_tax.parse::<f64>().unwrap_or(0.0),result.net_pay.parse::<f64>().unwrap_or(0.0),snapshot,status,config_version]).map_err(|e|e.to_string())?;
        conn.last_insert_rowid()
    };

    let component_lookup = |code: &str| -> Option<i64> { conn.query_row("SELECT id FROM salary_components WHERE code=?1", [code], |r| r.get(0)).ok() };
    for item in &result.items {
        if item.component_type == "metadata" { continue; }
        conn.execute("INSERT INTO payroll_items(payroll_record_id,component_id,component_name,component_code,component_type,calc_type,rate_or_value,formula_expression,amount) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id,component_lookup(&item.code),item.name,item.code,item.component_type,item.calc_type,item.rate,item.formula,item.amount.parse::<f64>().unwrap_or(0.0)]).map_err(|e|e.to_string())?;
    }
    Ok(id)
}

fn chrono_like_now() -> String { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_else(|_| "0".into()) }

fn records_for_period(conn: &rusqlite::Connection, period_id: i64) -> Vec<PayrollRecordRow> {
    let sql = r#"
    SELECT r.id,r.employee_id,e.employee_code,e.first_name||' '||e.last_name,d.name,p.title,
           r.base_salary,r.gross_earnings,r.total_deductions,r.total_tax,r.net_pay,
           COALESCE(json_extract(r.calculation_snapshot, '$.totals.employer_contributions'),0),
           r.status,r.config_version,r.calculation_snapshot
    FROM payroll_records r
    JOIN employees e ON e.id=r.employee_id
    LEFT JOIN departments d ON d.id=e.department_id
    LEFT JOIN positions p ON p.id=e.position_id
    WHERE r.period_id=?1 ORDER BY e.last_name,e.first_name
    "#;
    let mut stmt=match conn.prepare(sql){Ok(v)=>v,Err(_)=>return vec![]};
    stmt.query_map([period_id], |r| Ok(PayrollRecordRow{
        id:r.get(0)?,employee_id:r.get(1)?,employee_code:r.get(2)?,employee_name:r.get(3)?,department_name:r.get(4)?,position_title:r.get(5)?,base_salary:r.get(6)?,gross_earnings:r.get(7)?,total_deductions:r.get(8)?,total_tax:r.get(9)?,net_pay:r.get(10)?,employer_contributions:r.get(11)?,status:r.get(12)?,config_version:r.get(13)?,calculation_snapshot:r.get(14)?
    })).ok().map(|rows|rows.filter_map(|v|v.ok()).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn calculate_payroll_period(db: State<Database>, period_id: i64) -> PayrollRunResponse {
    let conn = db.conn.lock().unwrap();
    let period: (String,String,String,i64) = match conn.query_row("SELECT start_date,end_date,status,config_version FROM payroll_periods WHERE id=?1", [period_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))) { Ok(v)=>v, Err(e)=>return PayrollRunResponse{success:false,message:e.to_string(),period_id,records:vec![],errors:vec![e.to_string()]} };
    if period.2 == "closed" || period.2 == "locked" { return PayrollRunResponse{success:false,message:"This payroll period is already finalized.".into(),period_id,records:records_for_period(&conn,period_id),errors:vec![]}; }
    let rules = match active_rules(&conn){Ok(v)=>v,Err(e)=>return PayrollRunResponse{success:false,message:e.clone(),period_id,records:vec![],errors:vec![e]}};
    conn.execute("UPDATE payroll_periods SET status='processing',config_version=?1,updated_at=datetime('now') WHERE id=?2",params![current_config_version(&conn),period_id]).ok();
    // Always work from the unique active employee set. The database unique index
    // additionally guarantees one payroll record per employee per period.
    let employees: Vec<i64> = match conn.prepare("SELECT id FROM employees WHERE is_active=1 AND employment_status='active' ORDER BY id") {
        Ok(mut stmt) => stmt.query_map([], |r| r.get(0)).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let config_version = current_config_version(&conn);
    let mut errors=Vec::new();
    for employee_id in employees {
        match calculate_employee(&conn,employee_id,&period.0,&period.1,&rules){
            Ok(result)=>{ if let Err(e)=save_calculation_record(&conn,period_id,employee_id,config_version,&result,"calculated"){errors.push(format!("Employee {}: {}",employee_id,e));} }
            Err(e)=>errors.push(format!("Employee {}: {}",employee_id,e)),
        }
    }
    if !errors.is_empty() { conn.execute("UPDATE payroll_periods SET status='open',updated_at=datetime('now') WHERE id=?1",[period_id]).ok(); }
    else { conn.execute("UPDATE payroll_periods SET status='processing',updated_at=datetime('now') WHERE id=?1",[period_id]).ok(); }
    if errors.is_empty() { audit::log(&conn,"payroll_calculation_completed",Some("payroll_periods"),Some(period_id),Some("Payroll calculation completed successfully")); }
    PayrollRunResponse{success:errors.is_empty(),message:if errors.is_empty(){"Payroll calculated. Review before finalizing.".into()}else{"Payroll calculation completed with errors.".into()},period_id,records:records_for_period(&conn,period_id),errors}
}

#[tauri::command]
pub fn get_payroll_records(db: State<Database>, period_id: i64) -> Vec<PayrollRecordRow> {
    let conn=db.conn.lock().unwrap(); records_for_period(&conn,period_id)
}

#[tauri::command]
pub fn finalize_payroll_period(db: State<Database>, period_id: i64) -> PayrollRunResponse {
    let conn=db.conn.lock().unwrap();
    let status:String=match conn.query_row("SELECT status FROM payroll_periods WHERE id=?1",[period_id],|r|r.get(0)){Ok(v)=>v,Err(e)=>return PayrollRunResponse{success:false,message:e.to_string(),period_id,records:vec![],errors:vec![e.to_string()]}};
    if status=="closed" || status=="locked" { return PayrollRunResponse{success:false,message:"Payroll period already finalized.".into(),period_id,records:records_for_period(&conn,period_id),errors:vec![]}; }
    let count:i64=conn.query_row("SELECT COUNT(DISTINCT employee_id) FROM payroll_records WHERE period_id=?1",[period_id],|r|r.get(0)).unwrap_or(0);
    let expected:i64=conn.query_row("SELECT COUNT(*) FROM employees WHERE is_active=1 AND employment_status='active'",[],|r|r.get(0)).unwrap_or(0);
    if count==0 { return PayrollRunResponse{success:false,message:"Calculate payroll before finalizing.".into(),period_id,records:vec![],errors:vec!["No payroll records found.".into()]}; }
    if count != expected { return PayrollRunResponse{success:false,message:format!("Payroll is incomplete: {} of {} active employees have a payroll record.",count,expected),period_id,records:records_for_period(&conn,period_id),errors:vec!["Missing employee payroll records. Recalculate before finalizing.".into()]}; }
    conn.execute("UPDATE payroll_records SET status='finalized',updated_at=datetime('now') WHERE period_id=?1",[period_id]).ok();
    conn.execute("UPDATE payroll_periods SET status='closed',closed_at=datetime('now'),updated_at=datetime('now') WHERE id=?1",[period_id]).ok();
    audit::log(&conn,"payroll_closed",Some("payroll_periods"),Some(period_id),Some("Payroll period finalized and closed"));

    let record_ids: Vec<(i64,i64,String,String)> = match conn.prepare("SELECT r.id,r.employee_id,e.employee_code,printf('%s-%s',p.period_name,e.employee_code) FROM payroll_records r JOIN employees e ON e.id=r.employee_id JOIN payroll_periods p ON p.id=r.period_id WHERE r.period_id=?1") {
        Ok(mut stmt) => stmt.query_map([period_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let loan_rule_active: bool = conn.query_row("SELECT is_active FROM payroll_rules WHERE code='LOAN_DED' LIMIT 1", [], |r| r.get::<_, i64>(0)).unwrap_or(0) == 1;
    let loan_rows: Vec<(i64,i64,f64,i64,i64)> = if loan_rule_active {
        match conn.prepare("SELECT id,employee_id,installment_amount,paid_installments,total_installments FROM loans WHERE status='active' AND paid_installments<total_installments") {
            Ok(mut stmt) => match stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))) {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        }
    } else { vec![] };
    for (loan_id, employee_id, amount, paid_installments, total_installments) in loan_rows {
        let record_id: Option<i64> = conn.query_row("SELECT id FROM payroll_records WHERE period_id=?1 AND employee_id=?2 LIMIT 1", params![period_id,employee_id], |r| r.get(0)).optional().ok().flatten();
        if let Some(record_id) = record_id {
            let installment = paid_installments + 1;
            conn.execute("INSERT INTO loan_payments(loan_id,amount,payment_date,installment_number,payroll_record_id) VALUES(?1,?2,date('now'),?3,?4)", params![loan_id,amount,installment,record_id]).ok();
            let new_paid = installment;
            let new_status = if new_paid >= total_installments { "paid" } else { "active" };
            conn.execute("UPDATE loans SET paid_installments=?1,status=?2,updated_at=datetime('now') WHERE id=?3", params![new_paid,new_status,loan_id]).ok();
        }
    }

    for (record_id,employee_id,_employee_code,base_no) in record_ids {
        let existing:i64=conn.query_row("SELECT COUNT(*) FROM payslips WHERE payroll_record_id=?1",[record_id],|r|r.get(0)).unwrap_or(0);
        if existing==0 {
            let net:f64=conn.query_row("SELECT net_pay FROM payroll_records WHERE id=?1",[record_id],|r|r.get(0)).unwrap_or(0.0);
            let number=format!("PS-{}-{}",period_id,base_no.replace(' ',"-"));
            conn.execute("INSERT OR IGNORE INTO payslips(payroll_record_id,employee_id,period_id,payslip_number,net_pay) VALUES(?1,?2,?3,?4,?5)",params![record_id,employee_id,period_id,number,net]).ok();
        }
    }
    let records=records_for_period(&conn,period_id);
    PayrollRunResponse{success:true,message:"Payroll finalized and payslips prepared.".into(),period_id,records,errors:vec![]}
}

#[tauri::command]
pub fn get_loans(db: State<Database>) -> Vec<LoanRow> {
    let conn=db.conn.lock().unwrap();
    let sql="SELECT l.id,l.employee_id,e.first_name||' '||e.last_name,l.principal,l.interest_rate,l.total_amount,l.installment_amount,l.total_installments,l.paid_installments,MAX(0,l.total_amount-(l.installment_amount*l.paid_installments)),l.start_date,l.status FROM loans l JOIN employees e ON e.id=l.employee_id ORDER BY l.start_date DESC,l.id DESC";
    let mut stmt=match conn.prepare(sql){Ok(v)=>v,Err(_)=>return vec![]};
    stmt.query_map([],|r|Ok(LoanRow{id:r.get(0)?,employee_id:r.get(1)?,employee_name:r.get(2)?,principal:r.get(3)?,interest_rate:r.get(4)?,total_amount:r.get(5)?,installment_amount:r.get(6)?,total_installments:r.get(7)?,paid_installments:r.get(8)?,remaining_amount:r.get(9)?,start_date:r.get(10)?,status:r.get(11)?})).ok().map(|rows|rows.filter_map(|r|r.ok()).collect()).unwrap_or_default()
}

#[tauri::command]
pub fn save_loan(db: State<Database>, request: LoanRequest) -> PayrollPeriodResponse {
    if request.employee_id<=0 || request.principal<=0.0 || request.installment_amount<=0.0 || request.total_installments<=0 { return PayrollPeriodResponse{success:false,message:"Enter valid loan information.".into(),id:None}; }
    let conn=db.conn.lock().unwrap();
    let result=match request.id{Some(id)=>conn.execute("UPDATE loans SET employee_id=?1,principal=?2,interest_rate=?3,total_amount=?4,installment_amount=?5,total_installments=?6,start_date=?7,updated_at=datetime('now') WHERE id=?8",params![request.employee_id,request.principal,request.interest_rate,request.total_amount,request.installment_amount,request.total_installments,request.start_date,id]).map(|_|id),None=>conn.execute("INSERT INTO loans(employee_id,principal,interest_rate,total_amount,installment_amount,total_installments,start_date,status) VALUES(?1,?2,?3,?4,?5,?6,?7,'active')",params![request.employee_id,request.principal,request.interest_rate,request.total_amount,request.installment_amount,request.total_installments,request.start_date]).map(|_|conn.last_insert_rowid())};
    match result{Ok(id)=>{audit::log(&conn,if request.id.is_some(){"loan_updated"}else{"loan_created"},Some("loans"),Some(id),Some("Loan / advance saved"));PayrollPeriodResponse{success:true,message:"Loan saved.".into(),id:Some(id)}},Err(e)=>PayrollPeriodResponse{success:false,message:e.to_string(),id:None}}
}

#[tauri::command]
pub fn delete_loan(db: State<Database>, loan_id: i64) -> PayrollPeriodResponse {
    let conn = db.conn.lock().unwrap();
    let exists: i64 = conn.query_row("SELECT COUNT(*) FROM loans WHERE id=?1", [loan_id], |r| r.get(0)).unwrap_or(0);
    if exists == 0 { return PayrollPeriodResponse { success:false, message:"Loan / advance was not found.".into(), id:Some(loan_id) }; }
    let tx = match conn.unchecked_transaction() { Ok(v) => v, Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(loan_id) } };
    if let Err(e) = tx.execute("DELETE FROM loan_payments WHERE loan_id=?1", [loan_id]) { return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(loan_id) }; }
    if let Err(e) = tx.execute("DELETE FROM loans WHERE id=?1", [loan_id]) { return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(loan_id) }; }
    audit::log(&tx, "loan_deleted", Some("loans"), Some(loan_id), Some("Loan / advance and its payment history deleted"));
    if let Err(e) = tx.commit() { return PayrollPeriodResponse { success:false, message:e.to_string(), id:Some(loan_id) }; }
    PayrollPeriodResponse { success:true, message:"Loan / advance deleted.".into(), id:Some(loan_id) }
}

#[tauri::command]
pub fn clear_loans(db: State<Database>) -> PayrollPeriodResponse {
    let conn = db.conn.lock().unwrap();
    let tx = match conn.unchecked_transaction() { Ok(v) => v, Err(e) => return PayrollPeriodResponse { success:false, message:e.to_string(), id:None } };
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM loans", [], |r| r.get(0)).unwrap_or(0);
    if let Err(e) = tx.execute("DELETE FROM loan_payments", []) { return PayrollPeriodResponse { success:false, message:e.to_string(), id:None }; }
    if let Err(e) = tx.execute("DELETE FROM loans", []) { return PayrollPeriodResponse { success:false, message:e.to_string(), id:None }; }
    audit::log(&tx, "loans_cleared", Some("loans"), None, Some(&format!("All loans / advances deleted; {count} records removed")));
    if let Err(e) = tx.commit() { return PayrollPeriodResponse { success:false, message:e.to_string(), id:None }; }
    PayrollPeriodResponse { success:true, message:format!("All loans / advances deleted: {count} records removed."), id:None }
}

#[tauri::command]
pub fn get_payslips(db: State<Database>) -> Vec<PayslipRow> {
    let conn=db.conn.lock().unwrap();
    let sql="SELECT s.id,s.payroll_record_id,s.employee_id,e.first_name||' '||e.last_name,e.employee_code,p.period_name,s.payslip_number,s.net_pay,s.generated_at,r.calculation_snapshot FROM payslips s JOIN employees e ON e.id=s.employee_id JOIN payroll_periods p ON p.id=s.period_id JOIN payroll_records r ON r.id=s.payroll_record_id ORDER BY s.generated_at DESC";
    let mut stmt=match conn.prepare(sql){Ok(v)=>v,Err(_)=>return vec![]};
    stmt.query_map([],|r|Ok(PayslipRow{id:r.get(0)?,payroll_record_id:r.get(1)?,employee_id:r.get(2)?,employee_name:r.get(3)?,employee_code:r.get(4)?,period_name:r.get(5)?,payslip_number:r.get(6)?,net_pay:r.get(7)?,generated_at:r.get(8)?,calculation_snapshot:r.get(9)?})).ok().map(|rows|rows.filter_map(|r|r.ok()).collect()).unwrap_or_default()
}
