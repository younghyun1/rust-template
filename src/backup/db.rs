use std::path::{Path, PathBuf};

use anyhow::bail;
use tracing::{error, info};

use crate::config::config::Config;

pub async fn backup_db(config: &Config) -> anyhow::Result<PathBuf> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("db_{}_{}.dump", config.db_name, timestamp);
    let output_path = config.backup_temp_dir.join(&filename);

    info!(
        db_name = %config.db_name,
        db_host = %config.db_host,
        output = %output_path.display(),
        "Starting PostgreSQL backup"
    );

    let output = match tokio::process::Command::new("pg_dump")
        .arg("--format=custom")
        .arg("--host")
        .arg(&config.db_host)
        .arg("--port")
        .arg(config.db_port.to_string())
        .arg("--username")
        .arg(&config.db_username)
        .arg("--dbname")
        .arg(&config.db_name)
        .arg("--file")
        .arg(&output_path)
        .env("PGPASSWORD", &config.db_password)
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            error!(error = %e, "Failed to spawn pg_dump process");
            bail!("Failed to spawn pg_dump process: {}", e);
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!(
            exit_code = ?output.status.code(),
            stderr = %stderr,
            "pg_dump failed"
        );
        cleanup_temp_file(&output_path).await;
        bail!("pg_dump exited with status {}: {}", output.status, stderr);
    }

    let metadata = match tokio::fs::metadata(&output_path).await {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, path = %output_path.display(), "Failed to stat pg_dump output file");
            bail!(
                "Failed to stat pg_dump output file {}: {}",
                output_path.display(),
                e
            );
        }
    };

    info!(
        path = %output_path.display(),
        size_bytes = metadata.len(),
        "PostgreSQL backup completed"
    );

    Ok(output_path)
}

async fn cleanup_temp_file(path: &Path) {
    if let Err(e) = tokio::fs::remove_file(path).await
        && e.kind() != std::io::ErrorKind::NotFound
    {
        error!(
            error = %e,
            path = %path.display(),
            "Failed to clean up partial temp file after backup failure"
        );
    }
}
