fn main() {
    println!("cargo:rerun-if-env-changed=PAYROLL_UPDATE_ENDPOINT");
    println!("cargo:rerun-if-env-changed=PAYROLL_UPDATE_PUBKEY");
    tauri_build::build()
}
