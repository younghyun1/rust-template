use anyhow::bail;
use google_drive3::api::File as DriveFile;
use tracing::{error, info, warn};

use super::auth::DriveHub;

/// List all non-folder files in a Drive folder, handling pagination.
/// Returns files sorted by createdTime descending (newest first).
async fn list_all_files_in_folder(
    hub: &DriveHub,
    folder_id: &str,
) -> anyhow::Result<Vec<DriveFile>> {
    let query = format!(
        "'{}' in parents and trashed = false and mimeType != 'application/vnd.google-apps.folder'",
        folder_id
    );

    let mut all_files: Vec<DriveFile> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut request = hub
            .files()
            .list()
            .q(&query)
            .spaces("drive")
            .order_by("createdTime desc")
            .param("fields", "nextPageToken, files(id, name, createdTime)")
            .page_size(1000);

        if let Some(ref token) = page_token {
            request = request.page_token(token);
        }

        let (_, file_list) = match request.doit().await {
            Ok(r) => r,
            Err(e) => {
                error!(error = %e, folder_id = folder_id, "Failed to list files for pruning");
                bail!("Failed to list files in folder '{}': {}", folder_id, e);
            }
        };

        if let Some(files) = file_list.files {
            all_files.extend(files);
        }

        match file_list.next_page_token {
            Some(token) if !token.is_empty() => {
                page_token = Some(token);
            }
            _ => break,
        }
    }

    Ok(all_files)
}

/// Delete all but the `keep` newest files in the given Google Drive folder.
/// Returns the number of files deleted.
pub async fn prune_old_backups(
    hub: &DriveHub,
    folder_id: &str,
    keep: usize,
) -> anyhow::Result<u32> {
    let files = list_all_files_in_folder(hub, folder_id).await?;

    let total = files.len();
    if total <= keep {
        info!(
            folder_id = folder_id,
            total_files = total,
            keep = keep,
            "No files to prune"
        );
        return Ok(0);
    }

    let to_delete = &files[keep..];
    let mut deleted_count: u32 = 0;

    for file in to_delete {
        let file_id = match &file.id {
            Some(id) => id,
            None => {
                warn!("Skipping file with no ID during pruning");
                continue;
            }
        };
        let file_name = match &file.name {
            Some(name) => name.as_str(),
            None => "unknown",
        };

        info!(
            file_name = file_name,
            file_id = %file_id,
            "Deleting old backup"
        );

        match hub.files().delete(file_id).doit().await {
            Ok(_) => {
                deleted_count += 1;
            }
            Err(e) => {
                error!(
                    error = %e,
                    file_name = file_name,
                    file_id = %file_id,
                    "Failed to delete file during pruning"
                );
            }
        }
    }

    info!(
        folder_id = folder_id,
        deleted = deleted_count,
        kept = keep,
        total_before = total,
        "Pruning completed"
    );

    Ok(deleted_count)
}
