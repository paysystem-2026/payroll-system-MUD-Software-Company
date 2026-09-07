mod engine;
mod retention;
mod scheduler;

pub use engine::{backup_settings, create_backup, default_backup_dir, delete_backup, import_lan_backup, restore_backup, restore_backup_file, verify_backup};
pub use scheduler::start_scheduler;

pub const DEFAULT_BACKUP_FREQUENCY: &str = "daily";
pub const DEFAULT_BACKUP_TIME: &str = "02:00";
pub const DEFAULT_BACKUP_RETENTION: i64 = 7;
