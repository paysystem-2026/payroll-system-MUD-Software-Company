use serde::Serialize;
use tauri::State;
use crate::database::connection::Database;
use crate::security::session;
use crate::security::audit;

fn require_admin(conn: &rusqlite::Connection, token: &str) -> Result<i64, String> {
    if token.trim().is_empty() { return Err("Authentication required".into()); }
    let mut stmt = conn.prepare("SELECT admin_id, session_token, is_locked FROM admin_sessions WHERE expires_at > datetime('now') ORDER BY id DESC")
        .map_err(|e| format!("Unable to validate session: {e}"))?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?, r.get::<_,i64>(2)?)))
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

// ── Currency formatting (RWF) ───────────────────────────────────────

fn fmt_grouped_2(v: f64) -> String {
    let raw = format!("{:.2}", v);
    let (sign, digits) = if let Some(rest) = raw.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", raw.as_str())
    };
    let mut parts = digits.split('.');
    let integer = parts.next().unwrap_or("0");
    let fraction = parts.next().unwrap_or("00");
    let chars: Vec<char> = integer.chars().collect();
    let mut grouped = String::with_capacity(integer.len() + integer.len() / 3);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*ch);
    }
    format!("{}{}.{}", sign, grouped, fraction)
}

fn fmt_rwf(v: f64) -> String { format!("RWF {}", fmt_grouped_2(v)) }
fn fmt_rwf_plain(v: f64) -> String { fmt_grouped_2(v) }
fn fmt_grouped_0(v: f64) -> String {
    let raw = format!("{:.0}", v.round());
    let (sign, digits) = if let Some(rest) = raw.strip_prefix('-') { ("-", rest) } else { ("", raw.as_str()) };
    let chars: Vec<char> = digits.chars().collect();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 { grouped.push(','); }
        grouped.push(*ch);
    }
    format!("{}{}", sign, grouped)
}
fn fmt_rwf_report(v: f64) -> String { fmt_grouped_0(v) }

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86400) as i64;
    let tod = secs % 86400;
    let h = tod / 3600;
    let m = (tod % 3600) / 60;
    let s = tod % 60;
    let (yr, mo, dy) = days_to_date(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", yr, mo, dy, h, m, s)
}

fn days_to_date(d: i64) -> (i64, u32, u32) {
    let yr = 1970 + d / 365;
    let rem = d % 365;
    let mo = (rem / 30) as u32 + 1;
    let dy = (rem % 30) as u32 + 1;
    (yr, mo.min(12), dy.min(28))
}

// ── Report data structures ──────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ReportSummary {
    pub total_employees: i64, pub active_employees: i64, pub total_departments: i64,
    pub total_periods: i64, pub closed_periods: i64, pub total_payroll_runs: i64,
    pub total_gross_all: f64, pub total_net_all: f64, pub total_tax_all: f64,
    pub total_loans_active: i64, pub total_loan_outstanding: f64,
    pub total_leave_records: i64, pub pending_leaves: i64,
}

#[derive(Serialize, Clone)]
pub struct PayrollReportRow {
    pub period_id: i64, pub period_name: String, pub start_date: String, pub end_date: String,
    pub employee_count: i64, pub total_gross: f64, pub total_deductions: f64,
    pub total_tax: f64, pub total_net: f64, pub employer_contributions: f64,
}

#[derive(Serialize, Clone)]
pub struct PayrollDetailRow {
    pub record_id: i64, pub employee_id: i64, pub employee_code: String, pub employee_name: String,
    pub department_name: Option<String>, pub position_title: Option<String>,
    pub base_salary: f64, pub gross_earnings: f64, pub total_deductions: f64,
    pub total_tax: f64, pub net_pay: f64, pub employer_contributions: f64,
    pub status: String, pub period_name: String, pub calculation_snapshot: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct StaffReportRow {
    pub id: i64, pub employee_code: String, pub full_name: String, pub gender: Option<String>,
    pub department_name: Option<String>, pub position_title: Option<String>, pub grade: Option<String>,
    pub employment_status: String, pub hire_date: Option<String>, pub base_salary: f64,
    pub dependants: i64, pub contract_type_name: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LeaveReportRow {
    pub id: i64, pub employee_code: String, pub employee_name: String, pub leave_type_name: String,
    pub start_date: String, pub end_date: String, pub days: f64, pub status: String, pub reason: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LeaveSummaryRow {
    pub employee_id: i64, pub employee_code: String, pub employee_name: String,
    pub leave_type_name: String, pub entitled: f64, pub used: f64, pub remaining: f64,
}

#[derive(Serialize, Clone)]
pub struct LoanReportRow {
    pub id: i64, pub employee_code: String, pub employee_name: String, pub principal: f64,
    pub interest_rate: f64, pub total_amount: f64, pub installment_amount: f64,
    pub total_installments: i64, pub paid_installments: i64, pub remaining_amount: f64,
    pub start_date: String, pub status: String,
}

#[derive(Serialize, Clone)]
pub struct CumulativeReportRow {
    pub period_name: String, pub employee_count: i64, pub total_gross: f64,
    pub total_deductions: f64, pub total_tax: f64, pub total_net: f64,
    pub employer_contributions: f64, pub loan_deductions: f64,
}

#[derive(Serialize, Clone)]
pub struct ReportData<T: Serialize + Clone> {
    pub rows: Vec<T>, pub totals: Option<ReportTotals>, pub generated_at: String,
}

#[derive(Serialize, Clone)]
pub struct ReportTotals {
    pub total_gross: f64, pub total_deductions: f64, pub total_tax: f64,
    pub total_net: f64, pub employer_contributions: f64, pub count: i64,
}

#[derive(Serialize, Clone)]
pub struct ExportResult {
    pub success: bool, pub message: String, pub file_path: Option<String>,
}

// ── Tauri commands ───────────────────────────────────────────────────

#[tauri::command]
pub fn get_report_summary(db: State<Database>, token: String) -> Result<ReportSummary, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get::<_,i64>(0)).unwrap_or(0) };
    let sum = |sql: &str| -> f64 { conn.query_row(sql, [], |r| r.get::<_,f64>(0)).unwrap_or(0.0) };
    audit::log(&conn, "report_generated", Some("summary"), None, Some("Report summary viewed"));
    Ok(ReportSummary {
        total_employees: count("SELECT COUNT(*) FROM employees"),
        active_employees: count("SELECT COUNT(*) FROM employees WHERE is_active=1 AND employment_status='active'"),
        total_departments: count("SELECT COUNT(*) FROM departments"),
        total_periods: count("SELECT COUNT(*) FROM payroll_periods"),
        closed_periods: count("SELECT COUNT(*) FROM payroll_periods WHERE status='closed'"),
        total_payroll_runs: count("SELECT COUNT(*) FROM payroll_records"),
        total_gross_all: sum("SELECT COALESCE(SUM(gross_earnings),0) FROM payroll_records"),
        total_net_all: sum("SELECT COALESCE(SUM(net_pay),0) FROM payroll_records"),
        total_tax_all: sum("SELECT COALESCE(SUM(total_tax),0) FROM payroll_records"),
        total_loans_active: count("SELECT COUNT(*) FROM loans WHERE status='active'"),
        total_loan_outstanding: sum("SELECT COALESCE(SUM(total_amount - installment_amount*paid_installments),0) FROM loans WHERE status='active'"),
        total_leave_records: count("SELECT COUNT(*) FROM leave_records"),
        pending_leaves: count("SELECT COUNT(*) FROM leave_records WHERE status='pending'"),
    })
}

#[tauri::command]
pub fn get_payroll_report(db: State<Database>, token: String, period_id: Option<i64>) -> Result<ReportData<PayrollReportRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_payroll_report_inner(&conn, period_id)?;
    let detail = if let Some(pid) = period_id { format!("Payroll report for period {}", pid) } else { "Payroll report (all periods)".to_string() };
    audit::log(&conn, "report_generated", Some("payroll"), period_id, Some(&detail));
    Ok(data)
}

#[tauri::command]
pub fn get_payroll_detail_report(db: State<Database>, token: String, period_id: Option<i64>) -> Result<ReportData<PayrollDetailRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_payroll_detail_report_inner(&conn, period_id)?;
    let detail = if let Some(pid) = period_id { format!("Payroll detail for period {}", pid) } else { "Payroll detail (all periods)".to_string() };
    audit::log(&conn, "report_generated", Some("payroll_detail"), period_id, Some(&detail));
    Ok(data)
}

#[tauri::command]
pub fn get_staff_report(db: State<Database>, token: String, department_id: Option<i64>, status: Option<String>) -> Result<ReportData<StaffReportRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_staff_report_inner(&conn, department_id, status.clone())?;
    let mut detail = "Staff report".to_string();
    if let Some(d) = department_id { detail.push_str(&format!(" (department {})", d)); }
    if let Some(s) = &status { detail.push_str(&format!(" (status {})", s)); }
    audit::log(&conn, "report_generated", Some("staff"), None, Some(&detail));
    Ok(data)
}

#[tauri::command]
pub fn get_leave_report(db: State<Database>, token: String, year: Option<i64>, status: Option<String>) -> Result<ReportData<LeaveReportRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_leave_report_inner(&conn, year, status.clone())?;
    let mut detail = "Leave report".to_string();
    if let Some(y) = year { detail.push_str(&format!(" (year {})", y)); }
    if let Some(s) = &status { detail.push_str(&format!(" (status {})", s)); }
    audit::log(&conn, "report_generated", Some("leaves"), None, Some(&detail));
    Ok(data)
}

#[tauri::command]
pub fn get_leave_summary_report(db: State<Database>, token: String, year: i64) -> Result<ReportData<LeaveSummaryRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_leave_summary_report_inner(&conn, year)?;
    audit::log(&conn, "report_generated", Some("leave_summary"), None, Some(&format!("Leave summary for year {}", year)));
    Ok(data)
}

#[tauri::command]
pub fn get_loan_report(db: State<Database>, token: String, status: Option<String>) -> Result<ReportData<LoanReportRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_loan_report_inner(&conn, status.clone())?;
    let mut detail = "Loan report".to_string();
    if let Some(s) = &status { detail.push_str(&format!(" (status {})", s)); }
    audit::log(&conn, "report_generated", Some("loans"), None, Some(&detail));
    Ok(data)
}

#[tauri::command]
pub fn get_cumulative_report(db: State<Database>, token: String, start_date: Option<String>, end_date: Option<String>) -> Result<ReportData<CumulativeReportRow>, String> {
    let conn = db.conn.lock().map_err(|_| "Database lock unavailable".to_string())?;
    let _admin_id = require_admin(&conn, &token)?;
    let data = get_cumulative_report_inner(&conn, start_date.clone(), end_date.clone())?;
    let mut detail = "Cumulative report".to_string();
    if let Some(s) = &start_date { detail.push_str(&format!(" (from {})", s)); }
    if let Some(e) = &end_date { detail.push_str(&format!(" (to {})", e)); }
    audit::log(&conn, "report_generated", Some("cumulative"), None, Some(&detail));
    Ok(data)
}

// ── Export path helpers ─────────────────────────────────────────────

fn exports_dir() -> std::path::PathBuf {
    let home=std::env::var("HOME").or_else(|_|std::env::var("USERPROFILE")).unwrap_or_else(|_|".".into()); let base=std::path::PathBuf::from(home).join(".payroll-system"); let _=std::fs::create_dir_all(&base); let cfg=base.join("report-output-dir.txt"); if let Ok(raw)=std::fs::read_to_string(&cfg){let d=std::path::PathBuf::from(raw.trim());if d.is_dir(){return d;}} let d=base.join("exports");let _=std::fs::create_dir_all(&d);d
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}

fn write_csv(filename: &str, headers: &[&str], rows: &[Vec<String>]) -> Result<String, String> {
    let path = exports_dir().join(filename);
    let mut content = String::new();
    content.push_str(&headers.join(",")); content.push('\n');
    for row in rows {
        let escaped: Vec<String> = row.iter().map(|c| csv_escape(c)).collect();
        content.push_str(&escaped.join(",")); content.push('\n');
    }
    std::fs::write(&path, content).map_err(|e| format!("Unable to write CSV: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

// ── Report document metadata ─────────────────────────────────────────

#[derive(Clone)]
struct CompanyReportInfo {
    name: String,
    legal_name: Option<String>,
    address: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    website: Option<String>,
    tin_number: Option<String>,
    rssb_number: Option<String>,
    logo_path: Option<String>,
}

fn get_company_report_info(conn: &rusqlite::Connection) -> CompanyReportInfo {
    conn.query_row(
        "SELECT name,legal_name,address,phone,email,website,tin_number,rssb_number,logo_path FROM companies WHERE is_active=1 ORDER BY id LIMIT 1",
        [],
        |r| Ok(CompanyReportInfo {
            name: r.get(0)?, legal_name: r.get(1)?, address: r.get(2)?, phone: r.get(3)?,
            email: r.get(4)?, website: r.get(5)?, tin_number: r.get(6)?, rssb_number: r.get(7)?,
            logo_path: r.get(8)?,
        }),
    ).unwrap_or_else(|_| CompanyReportInfo {
        name: "Payroll System".into(), legal_name: None, address: None, phone: None,
        email: None, website: None, tin_number: None, rssb_number: None, logo_path: None,
    })
}

fn company_lines(company: &CompanyReportInfo) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(v) = &company.legal_name { if !v.trim().is_empty() && v != &company.name { lines.push(v.clone()); } }
    let mut contact = Vec::new();
    if let Some(v) = &company.address { if !v.trim().is_empty() { contact.push(v.clone()); } }
    if let Some(v) = &company.phone { if !v.trim().is_empty() { contact.push(format!("Tel: {}", v)); } }
    if let Some(v) = &company.email { if !v.trim().is_empty() { contact.push(format!("Email: {}", v)); } }
    if let Some(v) = &company.website { if !v.trim().is_empty() { contact.push(format!("Web: {}", v)); } }
    if !contact.is_empty() { lines.push(contact.join("  |  ")); }
    let mut ids = Vec::new();
    if let Some(v) = &company.tin_number { if !v.trim().is_empty() { ids.push(format!("TIN: {}", v)); } }
    if let Some(v) = &company.rssb_number { if !v.trim().is_empty() { ids.push(format!("RSSB: {}", v)); } }
    if !ids.is_empty() { lines.push(ids.join("  |  ")); }
    lines
}

fn resolve_logo_bytes(company: &CompanyReportInfo) -> Option<Vec<u8>> {
    if let Some(path) = &company.logo_path {
        let p = std::path::PathBuf::from(path);
        let candidates = if p.is_absolute() { vec![p] } else {
            vec![p.clone(), std::env::current_dir().ok().map(|d| d.join(&p)).unwrap_or(p.clone())]
        };
        for candidate in candidates { if let Ok(bytes) = std::fs::read(candidate) { return Some(bytes); } }
    }
    Some(include_bytes!("../../../public/gorilla-doctors.jpeg").to_vec())
}

fn safe_filename(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { out.push(ch.to_ascii_lowercase()); }
        else if ch.is_whitespace() { out.push('_'); }
    }
    if out.is_empty() { "report".into() } else { out }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#39;")
}

fn build_html_report(company: &CompanyReportInfo, title: &str, headers: &[&str], rows: &[Vec<String>], totals: Option<&ReportTotals>) -> String {
    let logo = resolve_logo_bytes(company).map(|b| format!("data:image/jpeg;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b)));
    let mut h = String::from("<!doctype html><html><head><meta charset='utf-8'><title>Report</title><style>");
    h.push_str("@page{size:A4 landscape;margin:10mm}*{box-sizing:border-box}body{font-family:Arial,sans-serif;color:#161616;margin:0;background:#fff;font-size:10px}.page{width:100%}.brand{display:flex;gap:14px;align-items:center;border-bottom:3px solid #4a8b3f;padding-bottom:9px}.brand img{width:52px;height:52px;object-fit:contain}.company{flex:1}.company h1{font-size:19px;margin:0 0 3px}.company p{margin:2px 0;color:#555;font-size:9px}.title{margin:12px 0 7px;font-size:16px}.meta{color:#666;font-size:9px;margin-bottom:8px}table{width:100%;border-collapse:collapse;table-layout:fixed;page-break-inside:auto}th{word-break:normal;overflow-wrap:anywhere}td{overflow-wrap:anywhere;word-break:normal}th{background:#4a8b3f;color:#fff;padding:6px;text-align:left;font-size:9px;overflow-wrap:anywhere}td{border:1px solid #d7d7d7;padding:5px;font-size:9px;vertical-align:top;overflow-wrap:anywhere;word-break:normal}tr:nth-child(even) td{background:#f7f8f7}.totals{margin-top:10px;border:1px solid #d7d7d7;padding:8px;display:flex;gap:22px;flex-wrap:wrap}.totals b{color:#315d2b}.footer{margin-top:15px;border-top:1px solid #ddd;padding-top:6px;color:#777;font-size:8px}@media print{.no-print{display:none!important}}");
    h.push_str("</style></head><body><main class='page'><header class='brand'>");
    if let Some(src) = logo { h.push_str(&format!("<img src='{}'>", src)); }
    h.push_str("<section class='company'>");
    h.push_str(&format!("<h1>{}</h1>", html_escape(&company.name)));
    for line in company_lines(company) { h.push_str(&format!("<p>{}</p>", html_escape(&line))); }
    h.push_str("</section></header>");
    h.push_str(&format!("<h2 class='title'>{}</h2><div class='meta'>Generated: {}</div><table><thead><tr>", html_escape(title), now_iso()));
    for hd in headers { h.push_str(&format!("<th>{}</th>", html_escape(hd))); }
    h.push_str("</tr></thead><tbody>");
    for row in rows { h.push_str("<tr>"); for cell in row { h.push_str(&format!("<td>{}</td>", html_escape(cell))); } h.push_str("</tr>"); }
    h.push_str("</tbody></table>");
    if let Some(t) = totals { h.push_str("<section class='totals'>");
        if t.count > 0 { h.push_str(&format!("<span><b>Count:</b> {}</span>", t.count)); }
        if t.total_gross != 0.0 { h.push_str(&format!("<span><b>Gross:</b> {}</span>", fmt_rwf(t.total_gross))); }
        if t.total_deductions != 0.0 { h.push_str(&format!("<span><b>Deductions:</b> {}</span>", fmt_rwf(t.total_deductions))); }
        if t.total_tax != 0.0 { h.push_str(&format!("<span><b>Tax:</b> {}</span>", fmt_rwf(t.total_tax))); }
        if t.total_net != 0.0 { h.push_str(&format!("<span><b>Net:</b> {}</span>", fmt_rwf(t.total_net))); }
        if t.employer_contributions != 0.0 { h.push_str(&format!("<span><b>Employer:</b> {}</span>", fmt_rwf(t.employer_contributions))); }
        h.push_str("</section>"); }
    h.push_str("<footer class='footer'>Generated by Payroll System | Official report | RWF</footer></main></body></html>");
    h
}

fn write_html_report(company: &CompanyReportInfo, title: &str, headers: &[&str], rows: &[Vec<String>], totals: Option<&ReportTotals>) -> Result<String, String> {
    let path = exports_dir().join(format!("{}.html", safe_filename(title)));
    std::fs::write(&path, build_html_report(company, title, headers, rows, totals)).map_err(|e| format!("Unable to write HTML report: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

// ── Valid A4 PDF writer ──────────────────────────────────────────────

struct PdfWriter { objects: Vec<Vec<u8>>, xref_offsets: Vec<usize> }
impl PdfWriter {
    fn new() -> Self { Self { objects: Vec::new(), xref_offsets: Vec::new() } }
    fn add(&mut self, body: Vec<u8>) -> usize { self.objects.push(body); self.objects.len() }
    fn set(&mut self, id: usize, body: Vec<u8>) { self.objects[id - 1] = body; }
    fn build(&mut self, root: usize) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(); self.xref_offsets.clear();
        for (i, obj) in self.objects.iter().enumerate() {
            self.xref_offsets.push(out.len()); out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes()); out.extend_from_slice(obj); out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len(); out.extend_from_slice(format!("xref\n0 {}\n", self.objects.len() + 1).as_bytes()); out.extend_from_slice(b"0000000000 65535 f \n");
        for off in &self.xref_offsets { out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes()); }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n", self.objects.len()+1, root, xref).as_bytes()); out
    }
}

fn pdf_escape(s: &str) -> String {
    let ascii: String = s.chars().map(|c| if c.is_ascii() { c } else { '?' }).collect();
    ascii.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

fn jpeg_size(data: &[u8]) -> Option<(u16,u16)> {
    if data.len() < 4 || data[0] != 0xff || data[1] != 0xd8 { return None; }
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xff { i += 1; continue; }
        while i < data.len() && data[i] == 0xff { i += 1; }
        if i >= data.len() { break; }
        let marker = data[i]; i += 1;
        if marker == 0xd8 || marker == 0xd9 { continue; }
        if i + 2 > data.len() { break; }
        let len = u16::from_be_bytes([data[i],data[i+1]]) as usize;
        if len < 2 || i + len > data.len() { break; }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) && len >= 7 {
            return Some((u16::from_be_bytes([data[i+5],data[i+6]]), u16::from_be_bytes([data[i+3],data[i+4]])));
        }
        i += len;
    }
    None
}

fn pdf_text(content: &mut String, font: &str, size: f64, x: f64, y: f64, text: &str) {
    content.push_str(&format!("BT /{} {:.1} Tf {:.1} {:.1} Td ({}) Tj ET\n", font, size, x, y, pdf_escape(text)));
}

fn wrap_text(s: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(4); let mut out = Vec::new(); let mut line = String::new();
    for word in s.split_whitespace() {
        if line.len() + word.len() + 1 > max_chars && !line.is_empty() { out.push(line); line = String::new(); }
        if !line.is_empty() { line.push(' '); } line.push_str(word);
    }
    if !line.is_empty() { out.push(line); } if out.is_empty() { out.push(String::new()); } out
}

fn report_column_widths(headers:&[&str],content_w:f64)->Vec<f64>{
    let n=headers.len();
    let normalize=|r:&[f64]|{let sum:f64=r.iter().sum();r.iter().map(|v|content_w*v/sum).collect::<Vec<_>>()};
    if n==18 && headers.first()==Some(&"No.") && headers.get(1)==Some(&"Employee Code") { return normalize(&[0.42,1.18,2.05,1.02,0.95,1.32,1.10,0.94,0.98,1.02,0.98,0.90,1.05,1.15,1.05,1.00,1.10,0.90]); }
    if n==12 && headers.first()==Some(&"No.") && headers.get(1)==Some(&"Code") { return normalize(&[0.65,1.10,1.75,1.55,1.35,1.20,1.05,1.05,1.25,1.20,1.20,1.35]); }
    if n==11 && headers.first()==Some(&"Code") && headers.get(1)==Some(&"Name") { return normalize(&[0.78,1.85,0.70,1.65,1.65,0.78,0.92,1.00,1.18,0.72,1.28]); }
    if n==11 && headers.first()==Some(&"Code") && headers.get(2)==Some(&"Principal") { return normalize(&[0.85,1.60,1.30,0.80,1.30,1.35,0.95,0.85,1.35,1.00,0.95]); }
    if n==9 && headers.first()==Some(&"Period") { return normalize(&[1.65,1.05,1.05,0.90,1.30,1.30,1.20,1.30,1.30]); }
    if n==8 && headers.first()==Some(&"Code") && headers.get(2)==Some(&"Leave Type") { return normalize(&[0.82,1.90,1.55,1.02,1.02,0.72,0.95,1.95]); }
    if n==8 && headers.first()==Some(&"Period") { return normalize(&[1.55,0.80,1.20,1.20,1.10,1.20,1.20,1.20]); }
    if n==6 && headers.first()==Some(&"Code") { return normalize(&[1.00,1.85,1.65,1.00,1.00,1.10]); }
    let mut w=Vec::with_capacity(n); for h in headers { let x=h.to_ascii_lowercase(); w.push(if x.contains("name")||x.contains("employee"){2.1}else if x.contains("department")||x.contains("position")||x.contains("reason"){1.65}else if x.contains("period")||x.contains("leave type"){1.45}else{1.0}); } normalize(&w)
}

fn write_pdf_report(company:&CompanyReportInfo,title:&str,headers:&[&str],rows:&[Vec<String>],totals:Option<&ReportTotals>)->Result<String,String>{
    let path=exports_dir().join(format!("{}.pdf",safe_filename(title)));
    let landscape=headers.len()>=8;
    let (pw,ph)=if landscape{(841.89_f64,595.28_f64)}else{(595.28_f64,841.89_f64)};
    let margin=if landscape{20.0_f64}else{34.0_f64};
    let cw=pw-margin*2.0;
    let widths=report_column_widths(headers,cw);
    let mut pdf=PdfWriter::new();
    let pages_id=pdf.add(Vec::new());
    let catalog_id=pdf.add(format!("<< /Type /Catalog /Pages {} 0 R >>",pages_id).into_bytes());
    let f1=pdf.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let f2=pdf.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    let logo=resolve_logo_bytes(company).filter(|b|jpeg_size(b).is_some());
    let logo_id=logo.as_ref().map(|b|{
        let(w,h)=jpeg_size(b).unwrap();
        pdf.add(format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",w,h,b.len()).into_bytes().into_iter().chain(b.iter().copied()).chain(b"\nendstream".iter().copied()).collect())
    });

    // A payroll register has many columns. Keep every cell inside its own column,
    // wrap text only at word boundaries, and keep numeric values right-aligned.
    let header_h=42.0_f64;
    let row_base=18.0_f64;
    let line_h=7.6_f64;
    let top=ph-margin-78.0;
    let bottom=38.0_f64;
    let usable=top-header_h-bottom-(if totals.is_some(){38.0}else{0.0});
    let is_numeric_col=|i:usize| { if headers.first()==Some(&"Code") && headers.get(2)==Some(&"Leave Type") { i==5 } else if headers.first()==Some(&"Code") && headers.get(1)==Some(&"Name") { i==8 || i==9 } else { i>=3 } };
    let cell_lines=|i:usize,cell:&str| -> Vec<String> {
        let max_chars=((widths[i]/4.0).floor() as usize).max(6);
        if is_numeric_col(i) {
            // Currency/amount cells should normally stay on one line. If an
            // unusually long value occurs, split at spaces rather than across words.
            let t=cell.trim();
            if t.chars().count()<=max_chars { vec![t.to_string()] } else { wrap_text(t,max_chars) }
        } else {
            wrap_text(cell.trim(),max_chars)
        }
    };
    let row_height=|row:&Vec<String>| -> f64 {
        let mut lines=1usize;
        for (i,cell) in row.iter().enumerate().take(headers.len()) {
            lines=lines.max(cell_lines(i,cell).len().max(1));
        }
        row_base+(lines.saturating_sub(1) as f64)*line_h
    };

    let mut chunks:Vec<Vec<Vec<String>>>=Vec::new();
    let mut chunk:Vec<Vec<String>>=Vec::new();
    let mut used=0.0_f64;
    for row in rows {
        let rh=row_height(row);
        if !chunk.is_empty() && used+rh>usable {
            chunks.push(chunk);
            chunk=Vec::new();
            used=0.0;
        }
        chunk.push(row.clone());
        used+=rh;
    }
    if !chunk.is_empty() || rows.is_empty(){chunks.push(chunk);}

    let mut page_ids=Vec::new();
    for (pi,prows) in chunks.iter().enumerate(){
        let mut c=String::new();
        let mut y=ph-margin;
        if logo_id.is_some(){c.push_str(&format!("q 42 0 0 42 {:.1} {:.1} cm /Im1 Do Q\n",margin,y-42.0));}
        pdf_text(&mut c,"F2",13.5,margin+52.0,y-13.0,&company.name);
        for(i,line)in company_lines(company).iter().take(2).enumerate(){pdf_text(&mut c,"F1",6.5,margin+52.0,y-25.0-(i as f64*8.0),line);}
        pdf_text(&mut c,"F2",10.0,margin,y-64.0,title);
        pdf_text(&mut c,"F1",6.3,pw-margin-18.0,y-64.0,&format!("{}",pi+1));
        y=top;

        // Header background and wrapped labels.
        c.push_str("0.29 0.55 0.25 rg\n");
        c.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re f\n",margin,y-header_h,cw,header_h));
        c.push_str("1 1 1 rg\n");
        let mut x=margin;
        for(i,h)in headers.iter().enumerate(){
            let lines=wrap_text(h,((widths[i]/4.0).floor() as usize).max(6));
            for(li,line)in lines.iter().enumerate().take(4){
                let tx=x+3.0;
                pdf_text(&mut c,"F2",5.7,tx,y-9.0-(li as f64*7.0),line);
            }
            x+=widths[i];
        }
        // Header column separators.
        c.push_str("0.75 0.75 0.75 RG 0.35 w\n");
        let mut vx=margin;
        for i in 0..headers.len().saturating_sub(1){
            vx+=widths[i];
            c.push_str(&format!("{:.2} {:.2} m {:.2} {:.2} l S\n",vx,y-header_h,vx,y));
        }
        y-=header_h;

        for row in prows {
            let rh=row_height(row);
            let is_total=row.get(2).map(|v|v.trim()=="TOTAL").unwrap_or(false);
            if is_total{
                c.push_str("0.90 0.94 0.89 rg\n");
                c.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re f\n",margin,y-rh,cw,rh));
            }
            c.push_str("0.72 0.72 0.72 RG 0.4 w\n");
            c.push_str(&format!("{:.2} {:.2} {:.2} {:.2} re S\n",margin,y-rh,cw,rh));
            let mut x=margin;
            for(i,cell)in row.iter().enumerate().take(headers.len()){
                let lines=cell_lines(i,cell);
                let fs=if headers.len()>=18{5.8}else if headers.len()>=9{6.35}else{7.0};
                for(li,line)in lines.iter().enumerate(){
                    let yy=y-11.0-(li as f64*line_h);
                    let right_aligned=is_numeric_col(i);
                    let tx=if right_aligned{
                        let tw=line.chars().count() as f64*fs*0.46;
                        (x+widths[i]-9.0-tw).max(x+4.0)
                    }else{x+4.0};
                    if headers.get(i)==Some(&"NET SALARY"){c.push_str("0.29 0.55 0.25 rg\n");}else{c.push_str("0 0 0 rg\n");}
                    pdf_text(&mut c,if is_total{"F2"}else{"F1"},fs,tx,yy,line);
                }
                x+=widths[i];
            }
            c.push_str("0.72 0.72 0.72 RG 0.35 w\n");
            let mut vx=margin;
            for i in 0..headers.len().saturating_sub(1){
                vx+=widths[i];
                c.push_str(&format!("{:.2} {:.2} m {:.2} {:.2} l S\n",vx,y-rh,vx,y));
            }
            y-=rh;
        }

        if pi+1==chunks.len(){
            if let Some(t)=totals{
                if y-36.0>bottom{
                    y-=6.0;
                    c.push_str("0.94 0.96 0.94 rg\n");
                    c.push_str(&format!("{:.2} {:.2} {:.2} 30 re f\n",margin,y-30.0,cw));
                    c.push_str("0 0 0 rg\n");
                    let vals=[format!("Count: {}",t.count),format!("Gross: {}",fmt_rwf(t.total_gross)),format!("Deductions: {}",fmt_rwf(t.total_deductions)),format!("Tax: {}",fmt_rwf(t.total_tax)),format!("Net: {}",fmt_rwf(t.total_net))];
                    let step=cw/5.0;
                    for(i,v)in vals.iter().enumerate(){pdf_text(&mut c,"F2",6.3,margin+3.0+i as f64*step,y-12.0,v);}
                }
            }
        }
        pdf_text(&mut c,"F1",5.8,margin,20.0,"Generated by Payroll System | Official report | RWF");
        let stream=c.into_bytes();
        let mut body=format!("<< /Length {} >>\nstream\n",stream.len()).into_bytes();
        body.extend_from_slice(&stream);body.extend_from_slice(b"\nendstream");
        let content=pdf.add(body);
        let res=if logo_id.is_some(){format!("<< /Font << /F1 {} 0 R /F2 {} 0 R >> /XObject << /Im1 {} 0 R >> >>",f1,f2,logo_id.unwrap())}else{format!("<< /Font << /F1 {} 0 R /F2 {} 0 R >> >>",f1,f2)};
        page_ids.push(pdf.add(format!("<< /Type /Page /Parent {} 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources {} /Contents {} 0 R >>",pages_id,pw,ph,res,content).into_bytes()));
    }
    let kids=page_ids.iter().map(|id|format!("{} 0 R",id)).collect::<Vec<_>>().join(" ");
    pdf.set(pages_id,format!("<< /Type /Pages /Kids [ {} ] /Count {} >>",kids,page_ids.len()).into_bytes());
    std::fs::write(&path,pdf.build(catalog_id)).map_err(|e|format!("Unable to write PDF: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

// ── XLSX export ─────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String { s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&apos;") }
fn crc32(data:&[u8])->u32 { let mut crc=0xffff_ffffu32; for &b in data { crc^=b as u32; for _ in 0..8 { crc=if crc&1!=0 {(crc>>1)^0xedb88320}else{crc>>1}; } } !crc }
fn col_letter(mut c:usize)->String { let mut s=String::new(); loop { s.insert(0,(b'A'+(c%26) as u8) as char); if c<26{break;} c=c/26-1;} s }
fn xlsx_number(cell:&str)->Option<String> { let n=cell.trim().replace("RWF ","").replace(',', ""); n.parse::<f64>().ok().map(|v|v.to_string()) }

fn write_xlsx_report(company:&CompanyReportInfo,title:&str,headers:&[&str],rows:&[Vec<String>],totals:Option<&ReportTotals>)->Result<String,String>{
    let path=exports_dir().join(format!("{}.xlsx",safe_filename(title))); let mut sheet=String::new(); sheet.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"6\" topLeftCell=\"A7\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews><sheetData>");
    let mut rn=1usize; let mut write_row=|cells:Vec<(String,u8)>|{ let mut s=format!("<row r=\"{}\">",rn); for (i,(v,style)) in cells.iter().enumerate(){ let col=col_letter(i); if let Some(n)=xlsx_number(v){s.push_str(&format!("<c r=\"{}{}\" s=\"{}\"><v>{}</v></c>",col,rn,style,n));}else{s.push_str(&format!("<c r=\"{}{}\" t=\"inlineStr\" s=\"{}\"><is><t>{}</t></is></c>",col,rn,style,xml_escape(v)));}} s.push_str("</row>"); sheet.push_str(&s); rn+=1;};
    write_row(vec![(company.name.clone(),3)]); for line in company_lines(company){write_row(vec![(line,0)]);} write_row(vec![(title.to_string(),3)]); write_row(vec![(format!("Generated: {}",now_iso()),0)]); write_row(headers.iter().map(|h|(h.to_string(),1)).collect());
    for row in rows { write_row(row.iter().map(|v|(v.clone(),if xlsx_number(v).is_some(){2}else{0})).collect()); }
    if let Some(t)=totals { write_row(vec![(format!("Count: {}",t.count),3)]); if t.total_gross!=0.0{write_row(vec![(format!("Total Gross: {}",fmt_rwf(t.total_gross)),3)]);} if t.total_deductions!=0.0{write_row(vec![(format!("Total Deductions: {}",fmt_rwf(t.total_deductions)),3)]);} if t.total_tax!=0.0{write_row(vec![(format!("Total Tax: {}",fmt_rwf(t.total_tax)),3)]);} if t.total_net!=0.0{write_row(vec![(format!("Total Net: {}",fmt_rwf(t.total_net)),3)]);} }
    sheet.push_str("</sheetData></worksheet>");
    let styles="<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"3\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><color rgb=\"FF315D2B\"/><name val=\"Calibri\"/></font></fonts><fills count=\"3\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF4A8B3F\"/></patternFill></fill><fill><patternFill patternType=\"solid\"><fgColor rgb=\"FFF0F4EF\"/></patternFill></fill></fills><borders count=\"2\"><border/><border><left style=\"thin\"/><right style=\"thin\"/><top style=\"thin\"/><bottom style=\"thin\"/></border></borders><cellXfs count=\"4\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"1\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"1\" borderId=\"1\" applyFill=\"1\"/><xf numFmtId=\"4\" fontId=\"0\" fillId=\"2\" borderId=\"1\"/><xf numFmtId=\"0\" fontId=\"2\" fillId=\"2\" borderId=\"1\"/></cellXfs></styleSheet>";
    let workbook="<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Report\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>";
    let rels="<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>";
    let ct="<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>";
    let root="<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";
    let entries=vec![("[Content_Types].xml".into(),ct.as_bytes().to_vec()),("_rels/.rels".into(),root.as_bytes().to_vec()),("xl/workbook.xml".into(),workbook.as_bytes().to_vec()),("xl/_rels/workbook.xml.rels".into(),rels.as_bytes().to_vec()),("xl/worksheets/sheet1.xml".into(),sheet.into_bytes()),("xl/styles.xml".into(),styles.as_bytes().to_vec())]; let bytes=build_zip(&entries); std::fs::write(&path,bytes).map_err(|e|format!("Unable to write XLSX: {e}"))?; Ok(path.to_string_lossy().to_string())
}
fn build_zip(entries:&[(String,Vec<u8>)])->Vec<u8>{let mut out=Vec::new();let mut central=Vec::new();let mut offsets=Vec::new();for(name,data)in entries{let off=out.len();let crc=crc32(data);out.extend_from_slice(&[80,75,3,4,20,0,0,0,0,0,0,0,0,0]);out.extend_from_slice(&crc.to_le_bytes());out.extend_from_slice(&(data.len()as u32).to_le_bytes());out.extend_from_slice(&(data.len()as u32).to_le_bytes());out.extend_from_slice(&(name.len()as u16).to_le_bytes());out.extend_from_slice(&[0,0]);out.extend_from_slice(name.as_bytes());out.extend_from_slice(data);offsets.push((name.clone(),off,data.len(),crc));}let cd=out.len();for(name,off,size,crc)in&offsets{central.extend_from_slice(&[80,75,1,2,20,0,20,0,0,0,0,0,0,0,0,0,0,0,0,0]);central.extend_from_slice(&crc.to_le_bytes());central.extend_from_slice(&(*size as u32).to_le_bytes());central.extend_from_slice(&(*size as u32).to_le_bytes());central.extend_from_slice(&(name.len()as u16).to_le_bytes());central.extend_from_slice(&[0,0,0,0,0,0,0,0,0,0,0,0]);central.extend_from_slice(&(*off as u32).to_le_bytes());central.extend_from_slice(name.as_bytes());}let csize=central.len();out.extend_from_slice(&central);out.extend_from_slice(&[80,75,5,6,0,0,0,0]);out.extend_from_slice(&(entries.len()as u16).to_le_bytes());out.extend_from_slice(&(entries.len()as u16).to_le_bytes());out.extend_from_slice(&(csize as u32).to_le_bytes());out.extend_from_slice(&(cd as u32).to_le_bytes());out.extend_from_slice(&[0,0]);out}

fn print_native(path: &std::path::Path) -> Result<(), String> {
    // Legacy print commands never bypass the application's print configuration.
    // They only open the real generated PDF; the configured print commands below
    // are responsible for sending a job to a selected printer.
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &format!("Start-Process -FilePath '{}'", path.display().to_string().replace('\'', "''"))])
            .status()
            .map_err(|e| format!("Unable to open report: {e}"))?;
        if status.success() { return Ok(()); }
        return Err("Unable to open the generated PDF on Windows.".into());
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open").arg(path).status()
            .map_err(|e| format!("Unable to open report on macOS: {e}"))?;
        if status.success() { return Ok(()); }
        return Err("Unable to open the generated PDF on macOS.".into());
    }
    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("xdg-open").arg(path).status()
            .map_err(|e| format!("Unable to open report. Install xdg-utils: {e}"))?;
        if status.success() { return Ok(()); }
        return Err("Unable to open the generated PDF. Install a PDF viewer.".into());
    }
}

#[derive(Serialize, Clone)]
pub struct PrinterInfo { pub name:String, pub is_default:bool }

#[tauri::command]
pub fn get_report_printers()->Result<Vec<PrinterInfo>,String>{
    #[cfg(target_os="linux")]
    { let out=std::process::Command::new("lpstat").arg("-p").output().map_err(|e|format!("Unable to list printers: {e}"))?; let def=std::process::Command::new("lpstat").arg("-d").output().ok().map(|o|String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default(); let mut v=Vec::new(); for line in String::from_utf8_lossy(&out.stdout).lines(){if let Some(rest)=line.strip_prefix("printer "){if let Some(name)=rest.split_whitespace().next(){v.push(PrinterInfo{name:name.to_string(),is_default:def.contains(name)});}}} return Ok(v); }
    #[cfg(target_os="macos")]
    { let out=std::process::Command::new("lpstat").arg("-a").output().map_err(|e|format!("Unable to list printers: {e}"))?; let def=std::process::Command::new("lpstat").arg("-d").output().ok().map(|o|String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default(); return Ok(String::from_utf8_lossy(&out.stdout).lines().filter_map(|line|line.split_whitespace().next()).map(|name|PrinterInfo{name:name.to_string(),is_default:def.contains(name)}).collect()); }
    #[cfg(target_os="windows")]
    { let out=std::process::Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command","Get-Printer | Select-Object -ExpandProperty Name"]).output().map_err(|e|format!("Unable to list printers: {e}"))?; let def=std::process::Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command","(Get-CimInstance Win32_Printer | Where-Object {$_.Default}).Name"]).output().ok().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default(); return Ok(String::from_utf8_lossy(&out.stdout).lines().filter(|x|!x.trim().is_empty()).map(|x|PrinterInfo{name:x.trim().to_string(),is_default:x.trim()==def}).collect()); }
}

fn report_output_config_path()->std::path::PathBuf{let home=std::env::var("HOME").or_else(|_|std::env::var("USERPROFILE")).unwrap_or_else(|_|".".into());std::path::PathBuf::from(home).join(".payroll-system").join("report-output-dir.txt")}

#[tauri::command]
pub fn get_report_output_dir()->Result<String,String>{let p=report_output_config_path();if let Ok(v)=std::fs::read_to_string(&p){let d=std::path::PathBuf::from(v.trim());if d.is_dir(){return Ok(d.to_string_lossy().to_string());}}Ok(exports_dir().to_string_lossy().to_string())}

#[tauri::command]
pub fn set_report_output_dir(db:State<Database>,token:String,path:String)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let d=std::path::PathBuf::from(path.trim());if !d.is_dir(){return Err("Selected report output folder does not exist.".into());}let cfg=report_output_config_path();if let Some(parent)=cfg.parent(){let _=std::fs::create_dir_all(parent);}std::fs::write(&cfg,d.to_string_lossy().as_bytes()).map_err(|e|format!("Unable to save report output location: {e}"))?;audit::log(&conn,"report_output_location_changed",Some("reports"),None,Some("Report output location changed"));Ok(export_result(d.to_string_lossy().to_string(),"Report output location saved"))}

fn print_pdf_to_printer(path:&std::path::Path,printer:Option<&str>,paper:Option<&str>,orientation:Option<&str>,copies:Option<u32>)->Result<(),String>{
    let printer=printer.map(str::trim).filter(|p|!p.is_empty()).ok_or_else(||"Please select a printer before printing.".to_string())?;
    let copies=copies.unwrap_or(1).clamp(1,99).to_string();
    let media=if paper.unwrap_or("A4").eq_ignore_ascii_case("LETTER"){"Letter"}else{"A4"};
    let landscape=orientation.unwrap_or("portrait").eq_ignore_ascii_case("landscape");
    #[cfg(any(target_os="linux",target_os="macos"))]
    {let mut cmd=std::process::Command::new("lp");cmd.args(["-d",printer,"-n",&copies,"-o",&format!("media={}",media)]);cmd.args(["-o",if landscape{"orientation-requested=4"}else{"orientation-requested=3"}]);cmd.arg(path);let out=cmd.output().map_err(|e|format!("Unable to start printing: {e}"))?;if out.status.success(){return Ok(());}let e=String::from_utf8_lossy(&out.stderr).trim().to_string();return Err(if e.is_empty(){"Printer rejected the print job.".into()}else{format!("Printing failed: {e}")});}
    #[cfg(target_os="windows")]
    {let escaped_path=path.display().to_string().replace('\\',"\\\\").replace('\'',"''");let escaped_printer=printer.replace('\'',"''");let st=std::process::Command::new("powershell").args(["-NoProfile","-NonInteractive","-Command",&format!("Start-Process -FilePath '{}' -Verb PrintTo -ArgumentList '{}'",escaped_path,escaped_printer)]).status().map_err(|e|format!("Unable to start Windows printing: {e}"))?;if st.success(){return Ok(());}return Err("Unable to start Windows print job.".into());}
}

#[tauri::command]
pub fn print_report_configured(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>,printer:Option<String>,paper:Option<String>,orientation:Option<String>,copies:Option<u32>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_report_table(&conn,&report_type,period_id,year,department_id,status,start_date,end_date)?;let pdf_totals=if report_type=="payroll-detail"{None}else{t.totals.as_ref()};
    let p=write_pdf_report(&c,t.title,&t.headers,&t.rows,pdf_totals)?;print_pdf_to_printer(std::path::Path::new(&p),printer.as_deref(),paper.as_deref(),orientation.as_deref(),copies)?;audit::log(&conn,"report_printed",Some("print"),None,Some("Configured report print sent"));Ok(export_result(p,"Report sent to selected printer"))}

#[tauri::command]
pub fn print_payslip_configured(db:State<Database>,token:String,payroll_record_id:i64,printer:Option<String>,paper:Option<String>,orientation:Option<String>,copies:Option<u32>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_payslip_table(&conn,payroll_record_id)?;let p=write_payslip_pdf(&c,&t.rows)?;print_pdf_to_printer(std::path::Path::new(&p),printer.as_deref(),paper.as_deref(),orientation.as_deref(),copies)?;audit::log(&conn,"payslip_printed",Some("print"),Some(payroll_record_id),Some("Configured payslip print sent"));Ok(export_result(p,"Payslip sent to selected printer"))}

// ── Report data builders ─────────────────────────────────────────────

fn get_payroll_report_inner(conn: &rusqlite::Connection, period_id: Option<i64>) -> Result<ReportData<PayrollReportRow>, String> {
    let sql = r#"
        SELECT p.id, p.period_name, p.start_date, p.end_date,
               COUNT(r.id), COALESCE(SUM(r.gross_earnings),0), COALESCE(SUM(r.total_deductions),0),
               COALESCE(SUM(r.total_tax),0), COALESCE(SUM(r.net_pay),0),
               COALESCE(SUM(COALESCE(json_extract(r.calculation_snapshot,'$.totals.employer_contributions'),0)),0)
        FROM payroll_periods p LEFT JOIN payroll_records r ON r.period_id = p.id
        WHERE (?1 IS NULL OR p.id = ?1) GROUP BY p.id ORDER BY p.start_date DESC, p.id DESC
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare payroll report: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![period_id], |r| Ok(PayrollReportRow {
        period_id: r.get(0)?, period_name: r.get(1)?, start_date: r.get(2)?, end_date: r.get(3)?,
        employee_count: r.get(4)?, total_gross: r.get(5)?, total_deductions: r.get(6)?,
        total_tax: r.get(7)?, total_net: r.get(8)?, employer_contributions: r.get(9)?,
    })).map_err(|e| format!("Unable to query payroll report: {e}"))?;
    let collected: Vec<PayrollReportRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals {
        total_gross: collected.iter().map(|r| r.total_gross).sum(),
        total_deductions: collected.iter().map(|r| r.total_deductions).sum(),
        total_tax: collected.iter().map(|r| r.total_tax).sum(),
        total_net: collected.iter().map(|r| r.total_net).sum(),
        employer_contributions: collected.iter().map(|r| r.employer_contributions).sum(),
        count: collected.iter().map(|r| r.employee_count).sum(),
    };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_payroll_detail_report_inner(conn: &rusqlite::Connection, period_id: Option<i64>) -> Result<ReportData<PayrollDetailRow>, String> {
    let sql = r#"
        SELECT r.id, r.employee_id, e.employee_code, e.first_name || ' ' || e.last_name,
               d.name, p.title, r.base_salary, r.gross_earnings, r.total_deductions, r.total_tax, r.net_pay,
               COALESCE(json_extract(r.calculation_snapshot,'$.totals.employer_contributions'),0),
               r.status, per.period_name, r.calculation_snapshot
        FROM payroll_records r JOIN employees e ON e.id = r.employee_id
        LEFT JOIN departments d ON d.id = e.department_id
        LEFT JOIN positions p ON p.id = e.position_id
        JOIN payroll_periods per ON per.id = r.period_id
        WHERE (?1 IS NULL OR r.period_id = ?1)
        ORDER BY per.period_name DESC, e.last_name, e.first_name
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare payroll detail: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![period_id], |r| Ok(PayrollDetailRow {
        record_id: r.get(0)?, employee_id: r.get(1)?, employee_code: r.get(2)?, employee_name: r.get(3)?,
        department_name: r.get(4)?, position_title: r.get(5)?, base_salary: r.get(6)?,
        gross_earnings: r.get(7)?, total_deductions: r.get(8)?, total_tax: r.get(9)?,
        net_pay: r.get(10)?, employer_contributions: r.get(11)?, status: r.get(12)?, period_name: r.get(13)?, calculation_snapshot: r.get(14)?,
    })).map_err(|e| format!("Unable to query payroll detail: {e}"))?;
    let collected: Vec<PayrollDetailRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals {
        total_gross: collected.iter().map(|r| r.gross_earnings).sum(),
        total_deductions: collected.iter().map(|r| r.total_deductions).sum(),
        total_tax: collected.iter().map(|r| r.total_tax).sum(),
        total_net: collected.iter().map(|r| r.net_pay).sum(),
        employer_contributions: collected.iter().map(|r| r.employer_contributions).sum(),
        count: collected.len() as i64,
    };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_staff_report_inner(conn: &rusqlite::Connection, department_id: Option<i64>, status: Option<String>) -> Result<ReportData<StaffReportRow>, String> {
    let sql = r#"
        SELECT e.id, e.employee_code, e.first_name || ' ' || e.last_name, e.gender, d.name, p.title, e.grade,
               e.employment_status, e.hire_date,
               COALESCE((SELECT base_salary FROM employee_salary_history s WHERE s.employee_id = e.id AND s.effective_date <= date('now') ORDER BY s.effective_date DESC, s.id DESC LIMIT 1), 0),
               e.dependants, ct.name
        FROM employees e
        LEFT JOIN departments d ON d.id = e.department_id
        LEFT JOIN positions p ON p.id = e.position_id
        LEFT JOIN contract_types ct ON ct.id = e.contract_type_id
        WHERE (?1 IS NULL OR e.department_id = ?1) AND (?2 IS NULL OR e.employment_status = ?2)
        ORDER BY e.last_name, e.first_name
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare staff report: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![department_id, status], |r| Ok(StaffReportRow {
        id: r.get(0)?, employee_code: r.get(1)?, full_name: r.get(2)?, gender: r.get(3)?,
        department_name: r.get(4)?, position_title: r.get(5)?, grade: r.get(6)?,
        employment_status: r.get(7)?, hire_date: r.get(8)?, base_salary: r.get(9)?,
        dependants: r.get(10)?, contract_type_name: r.get(11)?,
    })).map_err(|e| format!("Unable to query staff report: {e}"))?;
    let collected: Vec<StaffReportRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals {
        total_gross: collected.iter().map(|r| r.base_salary).sum(),
        total_deductions: 0.0, total_tax: 0.0,
        total_net: collected.iter().map(|r| r.base_salary).sum(),
        employer_contributions: 0.0, count: collected.len() as i64,
    };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_leave_report_inner(conn: &rusqlite::Connection, year: Option<i64>, status: Option<String>) -> Result<ReportData<LeaveReportRow>, String> {
    let sql = r#"
        SELECT l.id, e.employee_code, e.first_name || ' ' || e.last_name, t.name, l.start_date, l.end_date, l.days, l.status, l.reason
        FROM leave_records l JOIN employees e ON e.id = l.employee_id JOIN leave_types t ON t.id = l.leave_type_id
        WHERE (?1 IS NULL OR strftime('%Y', l.start_date) = ?2) AND (?3 IS NULL OR l.status = ?3)
        ORDER BY l.start_date DESC, l.id DESC
    "#;
    let year_str = year.map(|y| format!("{}", y));
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare leave report: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![year, year_str, status], |r| Ok(LeaveReportRow {
        id: r.get(0)?, employee_code: r.get(1)?, employee_name: r.get(2)?, leave_type_name: r.get(3)?,
        start_date: r.get(4)?, end_date: r.get(5)?, days: r.get(6)?, status: r.get(7)?, reason: r.get(8)?,
    })).map_err(|e| format!("Unable to query leave report: {e}"))?;
    let collected: Vec<LeaveReportRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals { total_gross: 0.0, total_deductions: 0.0, total_tax: 0.0, total_net: 0.0, employer_contributions: 0.0, count: collected.len() as i64 };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_leave_summary_report_inner(conn: &rusqlite::Connection, year: i64) -> Result<ReportData<LeaveSummaryRow>, String> {
    let sql = r#"
        SELECT e.id, e.employee_code, e.first_name || ' ' || e.last_name, t.name, t.default_days,
               COALESCE(SUM(CASE WHEN l.status='approved' AND strftime('%Y', l.start_date)=?1 THEN l.days ELSE 0 END), 0),
               t.default_days - COALESCE(SUM(CASE WHEN l.status='approved' AND strftime('%Y', l.start_date)=?1 THEN l.days ELSE 0 END), 0)
        FROM employees e CROSS JOIN leave_types t
        LEFT JOIN leave_records l ON l.employee_id = e.id AND l.leave_type_id = t.id
        WHERE e.is_active = 1 AND t.is_active = 1
        GROUP BY e.id, t.id ORDER BY e.last_name, e.first_name, t.name
    "#;
    let year_str = format!("{}", year);
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare leave summary: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![year_str], |r| Ok(LeaveSummaryRow {
        employee_id: r.get(0)?, employee_code: r.get(1)?, employee_name: r.get(2)?,
        leave_type_name: r.get(3)?, entitled: r.get(4)?, used: r.get(5)?, remaining: r.get(6)?,
    })).map_err(|e| format!("Unable to query leave summary: {e}"))?;
    let collected: Vec<LeaveSummaryRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals { total_gross: 0.0, total_deductions: 0.0, total_tax: 0.0, total_net: 0.0, employer_contributions: 0.0, count: collected.len() as i64 };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_loan_report_inner(conn: &rusqlite::Connection, status: Option<String>) -> Result<ReportData<LoanReportRow>, String> {
    let sql = r#"
        SELECT l.id, e.employee_code, e.first_name || ' ' || e.last_name, l.principal, l.interest_rate, l.total_amount,
               l.installment_amount, l.total_installments, l.paid_installments,
               MAX(0, l.total_amount - (l.installment_amount * l.paid_installments)), l.start_date, l.status
        FROM loans l JOIN employees e ON e.id = l.employee_id
        WHERE (?1 IS NULL OR l.status = ?1) ORDER BY l.start_date DESC, l.id DESC
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare loan report: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![status], |r| Ok(LoanReportRow {
        id: r.get(0)?, employee_code: r.get(1)?, employee_name: r.get(2)?, principal: r.get(3)?,
        interest_rate: r.get(4)?, total_amount: r.get(5)?, installment_amount: r.get(6)?,
        total_installments: r.get(7)?, paid_installments: r.get(8)?, remaining_amount: r.get(9)?,
        start_date: r.get(10)?, status: r.get(11)?,
    })).map_err(|e| format!("Unable to query loan report: {e}"))?;
    let collected: Vec<LoanReportRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals {
        total_gross: collected.iter().map(|r| r.total_amount).sum(),
        total_deductions: collected.iter().map(|r| r.installment_amount).sum(),
        total_tax: 0.0, total_net: collected.iter().map(|r| r.remaining_amount).sum(),
        employer_contributions: 0.0, count: collected.len() as i64,
    };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

fn get_cumulative_report_inner(conn: &rusqlite::Connection, start_date: Option<String>, end_date: Option<String>) -> Result<ReportData<CumulativeReportRow>, String> {
    let sql = r#"
        SELECT p.period_name, COUNT(r.id),
               COALESCE(SUM(r.gross_earnings),0), COALESCE(SUM(r.total_deductions),0),
               COALESCE(SUM(r.total_tax),0), COALESCE(SUM(r.net_pay),0),
               COALESCE(SUM(COALESCE(json_extract(r.calculation_snapshot,'$.totals.employer_contributions'),0)),0),
               COALESCE(SUM(COALESCE(json_extract(r.calculation_snapshot,'$.loan_deduction'),0)),0)
        FROM payroll_periods p LEFT JOIN payroll_records r ON r.period_id = p.id
        WHERE (?1 IS NULL OR p.start_date >= ?1) AND (?2 IS NULL OR p.end_date <= ?2)
        GROUP BY p.id ORDER BY p.start_date ASC, p.id ASC
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Unable to prepare cumulative report: {e}"))?;
    let rows = stmt.query_map(rusqlite::params![start_date, end_date], |r| Ok(CumulativeReportRow {
        period_name: r.get(0)?, employee_count: r.get(1)?, total_gross: r.get(2)?,
        total_deductions: r.get(3)?, total_tax: r.get(4)?, total_net: r.get(5)?,
        employer_contributions: r.get(6)?, loan_deductions: r.get(7)?,
    })).map_err(|e| format!("Unable to query cumulative report: {e}"))?;
    let collected: Vec<CumulativeReportRow> = rows.filter_map(|r| r.ok()).collect();
    let totals = ReportTotals {
        total_gross: collected.iter().map(|r| r.total_gross).sum(),
        total_deductions: collected.iter().map(|r| r.total_deductions).sum(),
        total_tax: collected.iter().map(|r| r.total_tax).sum(),
        total_net: collected.iter().map(|r| r.total_net).sum(),
        employer_contributions: collected.iter().map(|r| r.employer_contributions).sum(),
        count: collected.iter().map(|r| r.employee_count).sum(),
    };
    Ok(ReportData { rows: collected, totals: Some(totals), generated_at: now_iso() })
}

// ── Shared report table builder ───────────────────────────────────────

struct ReportTable {
    title: &'static str,
    headers: Vec<&'static str>,
    rows: Vec<Vec<String>>,
    totals: Option<ReportTotals>,
}

fn build_report_table(conn: &rusqlite::Connection, report_type: &str, period_id: Option<i64>, year: Option<i64>, department_id: Option<i64>, status: Option<String>, start_date: Option<String>, end_date: Option<String>) -> Result<ReportTable, String> {
    match report_type {
        "summary" => {
            let s = get_report_summary_data(conn)?;
            let headers = vec!["Metric", "Value"];
            let rows = vec![
                vec!["Total Employees".into(), s.total_employees.to_string()],
                vec!["Active Employees".into(), s.active_employees.to_string()],
                vec!["Departments".into(), s.total_departments.to_string()],
                vec!["Payroll Periods".into(), s.total_periods.to_string()],
                vec!["Closed Periods".into(), s.closed_periods.to_string()],
                vec!["Payroll Runs".into(), s.total_payroll_runs.to_string()],
                vec!["Total Gross".into(), fmt_rwf_plain(s.total_gross_all)],
                vec!["Total Net".into(), fmt_rwf_plain(s.total_net_all)],
                vec!["Total Tax".into(), fmt_rwf_plain(s.total_tax_all)],
                vec!["Active Loans".into(), s.total_loans_active.to_string()],
                vec!["Loan Outstanding".into(), fmt_rwf_plain(s.total_loan_outstanding)],
                vec!["Leave Records".into(), s.total_leave_records.to_string()],
                vec!["Pending Leaves".into(), s.pending_leaves.to_string()],
            ];
            Ok(ReportTable { title: "System Summary Report", headers, rows, totals: None })
        }
        "payroll" => { let d=get_payroll_report_inner(conn,period_id)?; let headers=vec!["Period","Start","End","Employees","Gross","Deductions","Tax","Net","Employer"]; let rows=d.rows.iter().map(|r|vec![r.period_name.clone(),r.start_date.clone(),r.end_date.clone(),r.employee_count.to_string(),fmt_rwf_plain(r.total_gross),fmt_rwf_plain(r.total_deductions),fmt_rwf_plain(r.total_tax),fmt_rwf_plain(r.total_net),fmt_rwf_plain(r.employer_contributions)]).collect(); Ok(ReportTable{title:"Payroll Report",headers,rows,totals:d.totals}) }
        "payroll-detail" => {
            let d=get_payroll_detail_report_inner(conn,period_id)?;
            // Keep the payroll register aligned with the approved monthly Excel structure:
            // identity + earnings/tax bases + statutory contributions + net + CHBI.
            // Values come from the frozen calculation snapshot, never from a second calculation.
            let codes=["BASIC","TRANSPORT","ACCOMMODATION","TAXABLE_BASE","PAYE","BASE_RSSB","PENSION_EMP","PENSION_ER","PENSION_2","PENSION_TOTAL","MATERNITY_EMP","MATERNITY_ER","MATERNITY_TOTAL","NET_SALARY","CHBI"];
            let _labels=["BASIC SALARY","TRANSPORT","ACCOMMODATION","BASE IMPOSABLE","PAYE (TPR)","BASE RSSB","PENSION 6% EMP","PENSION 6% ER","PENSION 2%","PENSION TOTAL","MATERNITY 0.3% EMP","MATERNITY 0.3% ER","MATERNITY TOTAL","NET SALARY","CHBI 0.5%"];
            let mut maps=Vec::<std::collections::HashMap<String,f64>>::new();
            for r in &d.rows {
                let mut map=std::collections::HashMap::new();
                if let Some(raw)=&r.calculation_snapshot {
                    if let Ok(v)=serde_json::from_str::<serde_json::Value>(raw) {
                        if let Some(items)=v.get("items").and_then(|x|x.as_array()) {
                            for item in items {
                                let code=item.get("code").and_then(|x|x.as_str()).unwrap_or("").to_uppercase();
                                let amount=item.get("amount").and_then(|x|x.as_f64().or_else(||x.as_str().and_then(|v|v.parse::<f64>().ok()))).unwrap_or(0.0);
                                map.insert(code,amount);
                            }
                        }
                    }
                }
                // The current rule set may not define separate BASE_RSSB/PENSION_TOTAL/MATERNITY_TOTAL
                // components. In that case the register derives the displayed bases/totals from the
                // same frozen snapshot values used by the payroll engine.
                let base_rssb=map.get("BASE_RSSB").copied().unwrap_or_else(||map.get("TAXABLE_BASE").copied().unwrap_or(0.0));
                map.insert("BASE_RSSB".into(),base_rssb);
                let pension_total=map.get("PENSION_TOTAL").copied().unwrap_or_else(||map.get("PENSION_EMP").copied().unwrap_or(0.0)+map.get("PENSION_ER").copied().unwrap_or(0.0)+map.get("PENSION_2").copied().unwrap_or(0.0));
                map.insert("PENSION_TOTAL".into(),pension_total);
                let maternity_total=map.get("MATERNITY_TOTAL").copied().unwrap_or_else(||map.get("MATERNITY_EMP").copied().unwrap_or(0.0)+map.get("MATERNITY_ER").copied().unwrap_or(0.0));
                map.insert("MATERNITY_TOTAL".into(),maternity_total);
                maps.push(map);
            }
            let headers=vec![
                "No.","Employee Code","Employee","BASIC SALARY","TRANSPORT","ACCOMMODATION","BASE IMPOSABLE","PAYE (TPR)",
                "BASE RSSB","PENSION 6% EMP","PENSION 6% ER","PENSION 2%","PENSION TOTAL",
                "MATERNITY 0.3% EMP","MATERNITY 0.3% ER","MATERNITY TOTAL","NET SALARY","CHBI 0.5%"
            ];
            let mut rows=Vec::new();
            for (i,r) in d.rows.iter().enumerate() {
                let mut row=vec![
                    (i+1).to_string(),
                    r.employee_code.clone(),
                    r.employee_name.clone(),
                ];
                for code in &codes[..15] {
                    let amount=*maps[i].get(*code).unwrap_or(&0.0);
                    row.push(if amount.abs()<0.000001{String::new()}else{fmt_rwf_report(amount)});
                }
                rows.push(row);
            }
            let mut total=vec!["".to_string(),"".to_string(),"TOTAL".to_string()];
            for code in &codes[..15] {
                let value=maps.iter().map(|m|m.get(*code).copied().unwrap_or(0.0)).sum::<f64>();
                total.push(if value.abs()<0.000001{String::new()}else{fmt_rwf_report(value)});
            }
            rows.push(total);
            let refs=headers.iter().map(|x|Box::leak(x.to_string().into_boxed_str())as &str).collect::<Vec<_>>();
            Ok(ReportTable{title:"GORILLA DOCTORS Payroll Register - Monthly Payroll Report",headers:refs,rows,totals:d.totals})
        }
"staff" => { let d=get_staff_report_inner(conn,department_id,status.clone())?; let headers=vec!["Code","Name","Gender","Department","Position","Grade","Status","Hire Date","Base Salary","Dependants","Contract"]; let rows=d.rows.iter().map(|r|vec![r.employee_code.clone(),r.full_name.clone(),r.gender.clone().unwrap_or_default(),r.department_name.clone().unwrap_or_default(),r.position_title.clone().unwrap_or_default(),r.grade.clone().unwrap_or_default(),r.employment_status.clone(),r.hire_date.clone().unwrap_or_default(),fmt_rwf_plain(r.base_salary),r.dependants.to_string(),r.contract_type_name.clone().unwrap_or_default()]).collect(); Ok(ReportTable{title:"Staff Report",headers,rows,totals:d.totals}) }
        "leaves" => { let d=get_leave_report_inner(conn,year,status.clone())?; let headers=vec!["Code","Name","Leave Type","Start","End","Days","Status","Reason"]; let rows=d.rows.iter().map(|r|vec![r.employee_code.clone(),r.employee_name.clone(),r.leave_type_name.clone(),r.start_date.clone(),r.end_date.clone(),format!("{:.1}",r.days),r.status.clone(),r.reason.clone().unwrap_or_default()]).collect(); Ok(ReportTable{title:"Leave Report",headers,rows,totals:d.totals}) }
        "leave-summary" => { let y=year.unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|(d.as_secs()/31536000+1970) as i64).unwrap_or(2026)); let d=get_leave_summary_report_inner(conn,y)?; let headers=vec!["Code","Name","Leave Type","Entitled","Used","Remaining"]; let rows=d.rows.iter().map(|r|vec![r.employee_code.clone(),r.employee_name.clone(),r.leave_type_name.clone(),format!("{:.1}",r.entitled),format!("{:.1}",r.used),format!("{:.1}",r.remaining)]).collect(); Ok(ReportTable{title:"Leave Summary Report",headers,rows,totals:d.totals}) }
        "loans" => { let d=get_loan_report_inner(conn,status.clone())?; let headers=vec!["Code","Name","Principal","Rate","Total","Installment","Total Inst.","Paid Inst.","Remaining","Start","Status"]; let rows=d.rows.iter().map(|r|vec![r.employee_code.clone(),r.employee_name.clone(),fmt_rwf_plain(r.principal),format!("{:.2}",r.interest_rate),fmt_rwf_plain(r.total_amount),fmt_rwf_plain(r.installment_amount),r.total_installments.to_string(),r.paid_installments.to_string(),fmt_rwf_plain(r.remaining_amount),r.start_date.clone(),r.status.clone()]).collect(); Ok(ReportTable{title:"Loan Report",headers,rows,totals:d.totals}) }
        "cumulative" => { let d=get_cumulative_report_inner(conn,start_date,end_date)?; let headers=vec!["Period","Employees","Gross","Deductions","Tax","Net","Employer","Loan Ded"]; let rows=d.rows.iter().map(|r|vec![r.period_name.clone(),r.employee_count.to_string(),fmt_rwf_plain(r.total_gross),fmt_rwf_plain(r.total_deductions),fmt_rwf_plain(r.total_tax),fmt_rwf_plain(r.total_net),fmt_rwf_plain(r.employer_contributions),fmt_rwf_plain(r.loan_deductions)]).collect(); Ok(ReportTable{title:"Cumulative Report",headers,rows,totals:d.totals}) }
        _ => Err(format!("Unknown report type: {}",report_type)),
    }
}

fn get_report_summary_data(conn: &rusqlite::Connection) -> Result<ReportSummary,String> {
    let count=|sql:&str|->i64{conn.query_row(sql,[],|r|r.get::<_,i64>(0)).unwrap_or(0)};
    let sum=|sql:&str|->f64{conn.query_row(sql,[],|r|r.get::<_,f64>(0)).unwrap_or(0.0)};
    Ok(ReportSummary{total_employees:count("SELECT COUNT(*) FROM employees"),active_employees:count("SELECT COUNT(*) FROM employees WHERE is_active=1 AND employment_status='active'"),total_departments:count("SELECT COUNT(*) FROM departments"),total_periods:count("SELECT COUNT(*) FROM payroll_periods"),closed_periods:count("SELECT COUNT(*) FROM payroll_periods WHERE status='closed'"),total_payroll_runs:count("SELECT COUNT(*) FROM payroll_records"),total_gross_all:sum("SELECT COALESCE(SUM(gross_earnings),0) FROM payroll_records"),total_net_all:sum("SELECT COALESCE(SUM(net_pay),0) FROM payroll_records"),total_tax_all:sum("SELECT COALESCE(SUM(total_tax),0) FROM payroll_records"),total_loans_active:count("SELECT COUNT(*) FROM loans WHERE status='active'"),total_loan_outstanding:sum("SELECT COALESCE(SUM(total_amount - installment_amount*paid_installments),0) FROM loans WHERE status='active'"),total_leave_records:count("SELECT COUNT(*) FROM leave_records"),pending_leaves:count("SELECT COUNT(*) FROM leave_records WHERE status='pending'")})
}



fn number_to_words_under_1000(n: u64) -> String {
    const ONES: [&str; 20] = ["zero","one","two","three","four","five","six","seven","eight","nine","ten","eleven","twelve","thirteen","fourteen","fifteen","sixteen","seventeen","eighteen","nineteen"];
    const TENS: [&str; 10] = ["","","twenty","thirty","forty","fifty","sixty","seventy","eighty","ninety"];
    if n < 20 { return ONES[n as usize].into(); }
    if n < 100 {
        return if n % 10 == 0 { TENS[(n/10) as usize].into() } else { format!("{} {}", TENS[(n/10) as usize], ONES[(n%10) as usize]) };
    }
    if n % 100 == 0 { return format!("{} hundred", ONES[(n/100) as usize]); }
    format!("{} hundred {}", ONES[(n/100) as usize], number_to_words_under_1000(n % 100))
}

fn number_to_words(n: i64) -> String {
    if n == 0 { return "zero".into(); }
    if n < 0 { return format!("minus {}", number_to_words(-n)); }
    let n = n as u64;
    let mut parts = Vec::new();
    let billions = n / 1_000_000_000;
    let millions = (n / 1_000_000) % 1000;
    let thousands = (n / 1000) % 1000;
    let rest = n % 1000;
    if billions > 0 { parts.push(format!("{} billion", number_to_words_under_1000(billions))); }
    if millions > 0 { parts.push(format!("{} million", number_to_words_under_1000(millions))); }
    if thousands > 0 { parts.push(format!("{} thousand", number_to_words_under_1000(thousands))); }
    if rest > 0 { parts.push(number_to_words_under_1000(rest)); }
    parts.join(" ")
}

fn payslip_words(amount: f64) -> String {
    format!("(A net of {} FRW)", number_to_words(amount.round() as i64))
}

fn row_value(rows: &[Vec<String>], label: &str) -> String {
    rows.iter().find(|r| r.first().map(|v| v == label).unwrap_or(false))
        .and_then(|r| r.get(1)).cloned().unwrap_or_default()
}

fn write_payslip_pdf(company: &CompanyReportInfo, rows: &[Vec<String>]) -> Result<String, String> {
    let path = exports_dir().join(format!("payslip_{}_{}.pdf", safe_filename(&row_value(rows, "Employee Code")), safe_filename(&row_value(rows, "Payroll Period"))));
    let (pw, ph) = (595.28_f64, 841.89_f64);
    let margin = 34.0_f64;
    let left = margin;
    let right = pw - margin;
    let top = ph - margin;
    let bottom = 48.0_f64;
    let width = right - left;
    let mut pdf = PdfWriter::new();
    let pages_id = pdf.add(Vec::new());
    let catalog_id = pdf.add(format!("<< /Type /Catalog /Pages {} 0 R >>", pages_id).into_bytes());
    let font_regular = pdf.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    let font_italic = pdf.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Italic /Encoding /WinAnsiEncoding >>".to_vec());
    let font_bold = pdf.add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());

    let logo_bytes = resolve_logo_bytes(company).filter(|b| jpeg_size(b).is_some());
    let logo_id = logo_bytes.as_ref().map(|b| {
        let (w,h)=jpeg_size(b).unwrap();
        let obj = format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",w,h,b.len())
            .into_bytes().into_iter().chain(b.iter().copied()).chain(b"\nendstream".iter().copied()).collect();
        pdf.add(obj)
    });

    let v = |label: &str| row_value(rows, label);
    let period = v("Payroll Period").to_uppercase();
    let code = v("Employee Code");
    let name = v("Employee Name");
    let department = v("Department");
    let position = v("Position");
    let grade = v("Grade");
    let hire_date = v("Hire Date");
    let dependants = v("Dependants");
    let rssb = v("RSSB Number");
    let bank = v("Bank");
    let account = v("Account");
    let paid = v("Number of days you are paid");
    let basic = v("Basic Salary");
    let transport = v("Transport Allowance");
    let housing = v("Housing Allowance");
    let other_payment = v("Other Payment");
    let gross = v("Gross Salary");
    let paye = v("Professional Tax");
    let pension = v("Caisse Sociale");
    let medical = v("Medical Insurance");
    let loan = v("Loan / Advance");
    let other_ded = v("Other Deductions");
    let total_ded = v("Total Deductions");
    let net = v("Net Remuneration");
    let annual = v("Annual Leave Entitlement");
    let used = rows.iter().find(|r| r.first().map(|x| x.starts_with("Leave Used")).unwrap_or(false)).and_then(|r| r.get(1)).cloned().unwrap_or_default();

    let mut c = String::new();
    // Outer payslip frame
    c.push_str("0 0 0 RG 0.8 w\n");
    c.push_str(&format!("{} {} {} {} re S\n", left, bottom, width, top-bottom));
    // Logo + centered title
    if let Some(_lid)=logo_id {
        c.push_str(&format!("q 46 0 0 46 {} {} cm /Im1 Do Q\n", left+7.0, top-58.0));
    }
    pdf_text(&mut c,"F2",13.0,left+125.0,top-27.0,&format!("PAYSLIP - {}",period));
    // Header fields
    let header_y = top - 82.0;
    c.push_str(&format!("{} {} m {} {} l S\n",left,header_y,right,header_y));
    let col2 = left + width*0.54;
    let info = [
        ("Employee Code",code,"RSSB Number",rssb),
        ("Employee Names",name.clone(),"Department",department),
        ("Position",position,"Grade",grade),
        ("Date of start",hire_date,"# of Dependant",dependants),
    ];
    for (i,(l1,x1,l2,x2)) in info.iter().enumerate() {
        let y=header_y-17.0-(i as f64*17.0);
        pdf_text(&mut c,"F3",8.5,left+2.0,y,&format!("{}:",l1));
        pdf_text(&mut c,"F1",8.5,left+74.0,y,x1);
        pdf_text(&mut c,"F3",8.5,col2,y,&format!("{}:",l2));
        pdf_text(&mut c,"F1",8.5,col2+82.0,y,x2);
    }
    let day_y=header_y-82.0;
    c.push_str(&format!("{} {} m {} {} l S\n",left,day_y+10.0,right,day_y+10.0));
    pdf_text(&mut c,"F3",8.5,left+2.0,day_y,&format!("Number of days you are paid: {}",paid));
    c.push_str(&format!("{} {} m {} {} l S\n",left,day_y-10.0,right,day_y-10.0));

    let mut y=day_y-30.0;
    let amount_x=right-8.0;
    let rows1=[("Basic Salary",basic),("Transport allowance",transport),("Housing Allowance",housing),("Other payment",other_payment),("Gross Salary",gross)];
    for (i,(label,val)) in rows1.iter().enumerate() {
        let yy=y-(i as f64*18.0);
        pdf_text(&mut c,"F3",8.8,left+2.0,yy,&format!("{}:",label));
        let tw=val.len() as f64*4.3;
        pdf_text(&mut c,if *label=="Gross Salary"{"F2"}else{"F1"},8.8,amount_x-tw,yy,val);
    }
    y -= 96.0;
    let rows2=[("Professional Tax",paye),("Caisse Sociale(6%)",pension),("Medical insurance agency",medical),("Other deductions",other_ded),("Total deductions",total_ded),("Net remuneration",net.clone())];
    for (i,(label,val)) in rows2.iter().enumerate() {
        let yy=y-(i as f64*18.0);
        pdf_text(&mut c,if *label=="Total deductions" || *label=="Net remuneration"{"F2"}else{"F3"},8.8,left+2.0,yy,&format!("{}:",label));
        let tw=val.len() as f64*4.3;
        pdf_text(&mut c,"F1",8.8,amount_x-tw,yy,val);
    }
    let words = payslip_words(net.replace(',',"").parse::<f64>().unwrap_or(0.0));
    pdf_text(&mut c,"F3",8.2,left+2.0,y-110.0,&words);

    let bank_y=y-143.0;
    pdf_text(&mut c,"F3",8.8,left+2.0,bank_y,&format!("Paid at: {}",bank));
    pdf_text(&mut c,"F3",8.8,left+2.0,bank_y-22.0,&format!("Account No: {}",account));
    pdf_text(&mut c,"F3",8.8,left+2.0,bank_y-58.0,&format!("Entitlement to annual leave: {}",annual));
    pdf_text(&mut c,"F3",8.8,left+2.0,bank_y-76.0,&format!("Leaves consumed from the beginning of the year: {}",used));

    let other_y=bank_y-112.0;
    c.push_str(&format!("{} {} m {} {} l S\n",left,other_y+13.0,right,other_y+13.0));
    pdf_text(&mut c,"F3",8.8,left+2.0,other_y,&format!("Other deductions: {}",if loan.is_empty()||loan=="0.00"||loan=="0"{"None".into()}else{format!("Loan / advance: {}",loan)}));

    let sig_y=bottom+45.0;
    c.push_str(&format!("{} {} m {} {} l S\n",left,sig_y+42.0,right,sig_y+42.0));
    pdf_text(&mut c,"F1",9.0,left+2.0,sig_y+25.0,"For MGVP");
    pdf_text(&mut c,"F1",9.0,right-130.0,sig_y+25.0,"For acceptance & reception");
    pdf_text(&mut c,"F1",8.0,right-130.0,sig_y+8.0,&name);
    c.push_str(&format!("{} {} m {} {} l S\n",left+5.0,sig_y-2.0,left+110.0,sig_y-2.0));
    c.push_str(&format!("{} {} m {} {} l S\n",right-140.0,sig_y-2.0,right-5.0,sig_y-2.0));
    pdf_text(&mut c,"F1",6.5,left,22.0,"Generated by Payroll System | Official payslip | RWF");

    let stream=c.into_bytes();
    let mut body=format!("<< /Length {} >>\nstream\n",stream.len()).into_bytes();
    body.extend_from_slice(&stream); body.extend_from_slice(b"\nendstream");
    let content_id=pdf.add(body);
    let resources=if logo_id.is_some(){format!("<< /Font << /F1 {} 0 R /F2 {} 0 R /F3 {} 0 R >> /XObject << /Im1 {} 0 R >> >>",font_regular,font_bold,font_italic,logo_id.unwrap())}else{format!("<< /Font << /F1 {} 0 R /F2 {} 0 R /F3 {} 0 R >> >>",font_regular,font_bold,font_italic)};
    let page_id=pdf.add(format!("<< /Type /Page /Parent {} 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources {} /Contents {} 0 R >>",pages_id,pw,ph,resources,content_id).into_bytes());
    pdf.set(pages_id,format!("<< /Type /Pages /Kids [ {} 0 R ] /Count 1 >>",page_id).into_bytes());
    std::fs::write(&path,pdf.build(catalog_id)).map_err(|e|format!("Unable to write payslip PDF: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

fn build_payslip_table(conn: &rusqlite::Connection, payroll_record_id: i64) -> Result<ReportTable, String> {
    let (employee_code, employee_name, department, position, grade, hire_date, dependants, rssb, bank, account, period_name, base, gross, deductions, tax, net, snapshot): (String,String,Option<String>,Option<String>,Option<String>,Option<String>,i64,Option<String>,Option<String>,Option<String>,String,f64,f64,f64,f64,f64,Option<String>) = conn.query_row(
        "SELECT e.employee_code,e.first_name||' '||e.last_name,d.name,p.title,e.grade,e.hire_date,e.dependants,e.rssb_number,e.bank_name,e.bank_account,pay.period_name,r.base_salary,r.gross_earnings,r.total_deductions,r.total_tax,r.net_pay,r.calculation_snapshot FROM payroll_records r JOIN employees e ON e.id=r.employee_id LEFT JOIN departments d ON d.id=e.department_id LEFT JOIN positions p ON p.id=e.position_id JOIN payroll_periods pay ON pay.id=r.period_id WHERE r.id=?1",
        [payroll_record_id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?,r.get(12)?,r.get(13)?,r.get(14)?,r.get(15)?,r.get(16)?)),
    ).map_err(|e| format!("Unable to load payslip: {e}"))?;
    let mut amounts = std::collections::HashMap::<String,f64>::new();
    let mut paid_days = String::new(); let mut scheduled_days = String::new();
    if let Some(raw) = snapshot {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(att) = v.get("attendance") {
                paid_days = att.get("paid_days").and_then(|x| x.as_i64()).map(|x| x.to_string()).unwrap_or_default();
                scheduled_days = att.get("scheduled_days").and_then(|x| x.as_i64()).map(|x| x.to_string()).unwrap_or_default();
            }
            if let Some(items) = v.get("items").and_then(|x| x.as_array()) {
                for item in items {
                    let code=item.get("code").and_then(|x|x.as_str()).unwrap_or("").to_uppercase();
                    let amount=item.get("amount").and_then(|x| {
                        x.as_f64().or_else(|| x.as_str().and_then(|v| v.parse::<f64>().ok()))
                    }).unwrap_or(0.0);
                    if !code.is_empty(){amounts.insert(code.clone(),amount);}
                }
            }
        }
    }
    // payroll_records are the frozen source of truth; snapshot items supply the component breakdown.
    let basic=amounts.get("BASIC").copied().unwrap_or(base);
    let transport=amounts.get("TRANSPORT").copied().unwrap_or(0.0);
    let accommodation=amounts.get("ACCOMMODATION").copied().unwrap_or(0.0);
    let other_payment=(gross-base.max(0.0)-transport-accommodation).max(0.0);
    let paye=amounts.get("PAYE").copied().unwrap_or(tax);
    let pension=amounts.get("PENSION_EMP").copied().unwrap_or(0.0);
    let maternity=amounts.get("MATERNITY_EMP").copied().unwrap_or(0.0);
    let loan=amounts.get("LOAN_DED").copied().unwrap_or(0.0);
    let other_ded=(deductions-paye-pension-maternity-loan).max(0.0);
    let year: i64 = conn.query_row("SELECT CAST(strftime('%Y',start_date) AS INTEGER) FROM payroll_periods WHERE id=(SELECT period_id FROM payroll_records WHERE id=?1)",[payroll_record_id],|r|r.get(0)).unwrap_or(2026);
    let annual: Option<(f64,f64)> = conn.query_row("SELECT t.default_days,COALESCE(SUM(CASE WHEN l.status='approved' THEN l.days ELSE 0 END),0) FROM leave_types t LEFT JOIN leave_records l ON l.leave_type_id=t.id AND l.employee_id=(SELECT employee_id FROM payroll_records WHERE id=?1) AND strftime('%Y',l.start_date)=?2 WHERE t.is_active=1 AND lower(t.name) LIKE '%annual%' GROUP BY t.id LIMIT 1",rusqlite::params![payroll_record_id,year.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).ok();
    let mut rows=Vec::new();
    rows.push(vec!["Employee Code".into(),employee_code]); rows.push(vec!["Employee Name".into(),employee_name]); rows.push(vec!["Department".into(),department.unwrap_or_default()]); rows.push(vec!["Position".into(),position.unwrap_or_default()]); rows.push(vec!["Grade".into(),grade.unwrap_or_default()]); rows.push(vec!["Hire Date".into(),hire_date.unwrap_or_default()]); rows.push(vec!["Dependants".into(),dependants.to_string()]); rows.push(vec!["RSSB Number".into(),rssb.unwrap_or_default()]); rows.push(vec!["Bank".into(),bank.unwrap_or_default()]); rows.push(vec!["Account".into(),account.unwrap_or_default()]); rows.push(vec!["Payroll Period".into(),period_name.clone()]); rows.push(vec!["Number of days you are paid".into(),if paid_days.is_empty(){"".into()}else{format!("{} out of {}",paid_days,if scheduled_days.is_empty(){"—".into()}else{scheduled_days})}]);
    rows.push(vec!["Basic Salary".into(),fmt_rwf_plain(basic)]); rows.push(vec!["Transport Allowance".into(),fmt_rwf_plain(transport)]); rows.push(vec!["Housing Allowance".into(),fmt_rwf_plain(accommodation)]); rows.push(vec!["Other Payment".into(),fmt_rwf_plain(other_payment)]); rows.push(vec!["Gross Salary".into(),fmt_rwf_plain(gross)]);
    rows.push(vec!["Professional Tax".into(),fmt_rwf_plain(paye)]); rows.push(vec!["Caisse Sociale".into(),fmt_rwf_plain(pension)]); rows.push(vec!["Medical Insurance".into(),fmt_rwf_plain(maternity)]); rows.push(vec!["Loan / Advance".into(),fmt_rwf_plain(loan)]); rows.push(vec!["Other Deductions".into(),fmt_rwf_plain(other_ded)]); rows.push(vec!["Total Deductions".into(),fmt_rwf_plain(deductions)]); rows.push(vec!["Net Remuneration".into(),fmt_rwf_plain(net)]);
    if let Some((entitled,used))=annual { rows.push(vec!["Annual Leave Entitlement".into(),format!("{:.1} days",entitled)]); rows.push(vec![format!("Leave Used ({})",year),format!("{:.1} days",used)]); rows.push(vec!["Leave Remaining".into(),format!("{:.1} days",(entitled-used).max(0.0))]); }
    Ok(ReportTable{title:"Payslip",headers:vec!["Item","Value"],rows,totals:Some(ReportTotals{total_gross:gross,total_deductions:deductions,total_tax:tax,total_net:net,employer_contributions:0.0,count:1})})
}

#[tauri::command]
pub fn export_payslip_pdf(db:State<Database>,token:String,payroll_record_id:i64)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_payslip_table(&conn,payroll_record_id)?;let p=write_payslip_pdf(&c,&t.rows)?;Ok(export_result(p,"Payslip PDF exported successfully"))}

#[tauri::command]
pub fn print_payslip(db:State<Database>,token:String,payroll_record_id:i64)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_payslip_table(&conn,payroll_record_id)?;let p=write_payslip_pdf(&c,&t.rows)?;print_native(std::path::Path::new(&p))?;audit::log(&conn,"payslip_printed",Some("print"),Some(payroll_record_id),Some("Native payslip print sent"));Ok(export_result(p,"Payslip sent to the default printer"))}

fn export_result(path:String,message:&str)->ExportResult{ExportResult{success:true,message:message.into(),file_path:Some(path)}}

#[tauri::command]
pub fn export_report_csv(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let t=build_report_table(&conn,&report_type,period_id,year,department_id,status.clone(),start_date.clone(),end_date.clone())?;let p=write_csv(&format!("{}.csv",safe_filename(t.title)),&t.headers,&t.rows)?;audit::log(&conn,"report_exported",Some("csv"),None,Some("CSV report exported"));Ok(export_result(p,"CSV exported successfully"))}

#[tauri::command]
pub fn export_report_html(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_report_table(&conn,&report_type,period_id,year,department_id,status.clone(),start_date.clone(),end_date.clone())?;let p=write_html_report(&c,t.title,&t.headers,&t.rows,t.totals.as_ref())?;Ok(export_result(p,"HTML report exported successfully"))}

#[tauri::command]
pub fn export_report_pdf(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_report_table(&conn,&report_type,period_id,year,department_id,status.clone(),start_date.clone(),end_date.clone())?;let pdf_totals=if report_type=="payroll-detail"{None}else{t.totals.as_ref()};
    let p=write_pdf_report(&c,t.title,&t.headers,&t.rows,pdf_totals)?;audit::log(&conn,"report_exported",Some("pdf"),None,Some("PDF report exported"));Ok(export_result(p,"PDF exported successfully"))}

#[tauri::command]
pub fn export_report_xlsx(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_report_table(&conn,&report_type,period_id,year,department_id,status.clone(),start_date.clone(),end_date.clone())?;let p=write_xlsx_report(&c,t.title,&t.headers,&t.rows,t.totals.as_ref())?;audit::log(&conn,"report_exported",Some("xlsx"),None,Some("Excel report exported"));Ok(export_result(p,"Excel workbook exported successfully"))}

#[tauri::command]
pub fn print_report(db:State<Database>,token:String,report_type:String,period_id:Option<i64>,year:Option<i64>,department_id:Option<i64>,status:Option<String>,start_date:Option<String>,end_date:Option<String>)->Result<ExportResult,String>{let conn=db.conn.lock().map_err(|_|"Database lock unavailable".to_string())?;require_admin(&conn,&token)?;let c=get_company_report_info(&conn);let t=build_report_table(&conn,&report_type,period_id,year,department_id,status.clone(),start_date.clone(),end_date.clone())?;let p=write_pdf_report(&c,t.title,&t.headers,&t.rows,t.totals.as_ref())?;print_native(std::path::Path::new(&p))?;audit::log(&conn,"report_printed",Some("print"),None,Some("Native report print sent"));Ok(export_result(p,"Report sent to the default printer"))}
