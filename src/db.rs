use mysql::prelude::*;
use mysql::Pool;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub deleted_at: String,
}

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/disk_viewer".to_string())
}

fn connect() -> Result<mysql::PooledConn, String> {
    let pool = Pool::new(db_url().as_str()).map_err(|e| format!("MySQL connection failed: {}", e))?;
    let mut conn = pool
        .get_conn()
        .map_err(|e| format!("MySQL connection failed: {}", e))?;
    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS delete_log (
            id INT AUTO_INCREMENT PRIMARY KEY,
            file_name VARCHAR(255),
            file_path VARCHAR(1000),
            size_bytes BIGINT,
            deleted_at VARCHAR(30)
        )",
    )
    .map_err(|e| format!("MySQL error: {}", e))?;
    Ok(conn)
}

pub fn log_delete(name: &str, path: &str, size: u64) -> Result<(), String> {
    let mut conn = connect()?;
    conn.exec_drop(
        "INSERT INTO delete_log (file_name, file_path, size_bytes, deleted_at)
         VALUES (?, ?, ?, NOW())",
        (name, path, size),
    )
    .map_err(|e| format!("MySQL error: {}", e))
}

pub fn load_history() -> Result<Vec<LogEntry>, String> {
    let mut conn = connect()?;
    conn.query_map(
        "SELECT file_name, file_path, size_bytes, deleted_at
         FROM delete_log ORDER BY id DESC",
        |(file_name, file_path, size_bytes, deleted_at): (String, String, u64, String)| LogEntry {
            file_name,
            file_path,
            size_bytes,
            deleted_at,
        },
    )
    .map_err(|e| format!("MySQL error: {}", e))
}