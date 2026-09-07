#[tauri::command]
pub fn ping() -> String {
    "payroll-system-backend-ready".to_string()
}
