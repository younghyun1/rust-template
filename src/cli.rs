use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "db-backup-goog",
    about = "Backup PostgreSQL and Minecraft server to Google Drive"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Backup PostgreSQL database and upload to Google Drive
    Db,
    /// Backup Minecraft server and upload to Google Drive
    Minecraft,
    /// Run all backups (db + minecraft) and prune old Minecraft backups
    All,
    /// Prune old Minecraft backups from Google Drive (keep N newest)
    Prune,
}
