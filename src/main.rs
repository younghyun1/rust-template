#![feature(const_type_name)]

use std::process::ExitCode;

use clap::Parser;
use tracing::{error, info};

use crate::cli::{Cli, Command};
use crate::config::config::Config;
use crate::setup_logger::setup_logger;

pub mod backup;
pub mod build_info;
pub mod cli;
pub mod config;
pub mod drive;
pub mod setup_logger;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let app_start_time = tokio::time::Instant::now();

    let (_log_guard, _stdout_guard) = setup_logger().await;
    let _span_entered = tracing::info_span!(std::any::type_name_of_val(&main)).entered();

    // Log panics via tracing before the process aborts
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => s.to_string(),
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "unknown panic payload".to_string(),
            },
        };
        let location = match panic_info.location() {
            Some(loc) => format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
            None => "unknown location".to_string(),
        };
        error!(
            panic_message = %payload,
            location = %location,
            "Process panicked"
        );
    }));

    info!(duration = ?app_start_time.elapsed(), "Logger initialized!");

    let cli = Cli::parse();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to load configuration");
            return ExitCode::FAILURE;
        }
    };

    // Ensure temp directory exists
    if let Err(e) = tokio::fs::create_dir_all(&config.backup_temp_dir).await {
        error!(
            error = %e,
            path = %config.backup_temp_dir.display(),
            "Failed to create backup temp directory"
        );
        return ExitCode::FAILURE;
    }

    let result = match cli.command {
        Command::Db => run_db_backup(&config).await,
        Command::Minecraft => run_minecraft_backup(&config).await,
        Command::All => run_all(&config).await,
        Command::Prune => run_prune(&config).await,
    };

    match result {
        Ok(()) => {
            info!(duration = ?app_start_time.elapsed(), "All operations completed successfully");
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, duration = ?app_start_time.elapsed(), "Operation failed");
            ExitCode::FAILURE
        }
    }
}

async fn run_db_backup(config: &Config) -> anyhow::Result<()> {
    let hub = drive::auth::build_hub(&config.google_credentials_path).await?;
    let folder_id =
        drive::upload::find_or_create_folder(&hub, &config.google_drive_folder_id, "DB_Backups")
            .await?;

    let dump_path = backup::db::backup_db(config).await?;
    drive::upload::upload_file(&hub, &folder_id, &dump_path).await?;

    // Clean up temp file after successful upload
    if let Err(e) = tokio::fs::remove_file(&dump_path).await {
        error!(
            error = %e,
            path = %dump_path.display(),
            "Failed to remove temp file after upload"
        );
    }

    Ok(())
}

async fn run_minecraft_backup(config: &Config) -> anyhow::Result<()> {
    let hub = drive::auth::build_hub(&config.google_credentials_path).await?;
    let folder_id = drive::upload::find_or_create_folder(
        &hub,
        &config.google_drive_folder_id,
        "Minecraft_Backups",
    )
    .await?;

    let archive_path = backup::minecraft::backup_minecraft(config).await?;
    drive::upload::upload_file(&hub, &folder_id, &archive_path).await?;

    // Prune old backups after successful upload
    drive::prune::prune_old_backups(&hub, &folder_id, config.mc_retention_count).await?;

    // Clean up temp file
    if let Err(e) = tokio::fs::remove_file(&archive_path).await {
        error!(
            error = %e,
            path = %archive_path.display(),
            "Failed to remove temp file after upload"
        );
    }

    Ok(())
}

async fn run_all(config: &Config) -> anyhow::Result<()> {
    let hub = drive::auth::build_hub(&config.google_credentials_path).await?;

    // --- DB backup ---
    let db_folder_id =
        drive::upload::find_or_create_folder(&hub, &config.google_drive_folder_id, "DB_Backups")
            .await?;

    let dump_path = backup::db::backup_db(config).await?;
    drive::upload::upload_file(&hub, &db_folder_id, &dump_path).await?;

    if let Err(e) = tokio::fs::remove_file(&dump_path).await {
        error!(
            error = %e,
            path = %dump_path.display(),
            "Failed to remove temp DB dump after upload"
        );
    }

    // --- Minecraft backup ---
    let mc_folder_id = drive::upload::find_or_create_folder(
        &hub,
        &config.google_drive_folder_id,
        "Minecraft_Backups",
    )
    .await?;

    let archive_path = backup::minecraft::backup_minecraft(config).await?;
    drive::upload::upload_file(&hub, &mc_folder_id, &archive_path).await?;

    // Prune old Minecraft backups
    drive::prune::prune_old_backups(&hub, &mc_folder_id, config.mc_retention_count).await?;

    if let Err(e) = tokio::fs::remove_file(&archive_path).await {
        error!(
            error = %e,
            path = %archive_path.display(),
            "Failed to remove temp Minecraft archive after upload"
        );
    }

    Ok(())
}

async fn run_prune(config: &Config) -> anyhow::Result<()> {
    let hub = drive::auth::build_hub(&config.google_credentials_path).await?;
    let folder_id = drive::upload::find_or_create_folder(
        &hub,
        &config.google_drive_folder_id,
        "Minecraft_Backups",
    )
    .await?;

    drive::prune::prune_old_backups(&hub, &folder_id, config.mc_retention_count).await?;

    Ok(())
}
