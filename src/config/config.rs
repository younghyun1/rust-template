use anyhow::bail;
use std::path::PathBuf;
use tracing::error;

pub struct Config {
    pub db_host: String,
    pub db_username: String,
    pub db_password: String,
    pub db_name: String,
    pub db_port: u16,
    pub minecraft_server_path: PathBuf,
    pub backup_temp_dir: PathBuf,
    pub mc_retention_count: usize,
    pub google_credentials_path: PathBuf,
    pub google_drive_folder_id: String,
}

fn require_env(key: &str) -> anyhow::Result<String> {
    match std::env::var(key) {
        Ok(val) => Ok(val),
        Err(e) => {
            error!(key = key, error = %e, "Required environment variable not set");
            bail!("Required environment variable '{}' not set: {}", key, e);
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        if let Err(e) = dotenvy::dotenv() {
            tracing::warn!(error = %e, "Failed to load .env file, continuing with existing environment");
        }

        let db_port_str = require_env("DB_PORT")?;
        let db_port: u16 = match db_port_str.parse() {
            Ok(port) => port,
            Err(e) => {
                error!(value = %db_port_str, error = %e, "DB_PORT is not a valid u16");
                bail!("DB_PORT '{}' is not a valid u16: {}", db_port_str, e);
            }
        };

        let mc_retention_str =
            std::env::var("MC_RETENTION_COUNT").unwrap_or_else(|_| "3".to_string());
        let mc_retention_count: usize = match mc_retention_str.parse() {
            Ok(count) => count,
            Err(e) => {
                error!(value = %mc_retention_str, error = %e, "MC_RETENTION_COUNT is not a valid usize");
                bail!(
                    "MC_RETENTION_COUNT '{}' is not a valid usize: {}",
                    mc_retention_str,
                    e
                );
            }
        };

        let backup_temp_dir = PathBuf::from(
            std::env::var("BACKUP_TEMP_DIR").unwrap_or_else(|_| "/tmp/db-backup-goog".to_string()),
        );

        let minecraft_server_path = PathBuf::from(require_env("MINECRAFT_SERVER_PATH")?);
        let google_credentials_path = PathBuf::from(require_env("GOOGLE_CREDENTIALS_PATH")?);
        let google_drive_folder_id = require_env("GOOGLE_DRIVE_FOLDER_ID")?;

        Ok(Config {
            db_host: require_env("DB_HOST")?,
            db_username: require_env("DB_USERNAME")?,
            db_password: require_env("DB_PASSWORD")?,
            db_name: require_env("DB_NAME")?,
            db_port,
            minecraft_server_path,
            backup_temp_dir,
            mc_retention_count,
            google_credentials_path,
            google_drive_folder_id,
        })
    }
}
