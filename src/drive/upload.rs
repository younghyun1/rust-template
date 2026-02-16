use std::io::BufReader;
use std::path::Path;

use anyhow::bail;
use google_drive3::api::{File as DriveFile, Scope};
use tracing::{error, info};

use super::auth::DriveHub;

/// Find an existing subfolder by name under `parent_id`, or create it if missing.
pub async fn find_or_create_folder(
    hub: &DriveHub,
    parent_id: &str,
    name: &str,
) -> anyhow::Result<String> {
    // Escape single quotes in folder name to prevent Drive API query injection
    let escaped_name = name.replace('\\', "\\\\").replace('\'', "\\'");
    let query = format!(
        "name = '{}' and '{}' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
        escaped_name, parent_id
    );

    let result = hub
        .files()
        .list()
        .q(&query)
        .spaces("drive")
        .param("fields", "files(id, name)")
        .add_scope(Scope::Full)
        .doit()
        .await;

    let (_, file_list) = match result {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, folder_name = name, "Failed to search for folder on Google Drive");
            bail!("Failed to search for folder '{}': {}", name, e);
        }
    };

    if let Some(files) = file_list.files
        && let Some(existing) = files.first()
        && let Some(ref id) = existing.id
    {
        info!(folder_name = name, folder_id = %id, "Found existing Drive folder");
        return Ok(id.clone());
    }

    // Folder doesn't exist - create it
    info!(
        folder_name = name,
        parent_id = parent_id,
        "Creating new Drive folder"
    );

    let folder_metadata = DriveFile {
        name: Some(name.to_string()),
        mime_type: Some("application/vnd.google-apps.folder".to_string()),
        parents: Some(vec![parent_id.to_string()]),
        ..Default::default()
    };

    let folder_mime: mime::Mime = match "application/vnd.google-apps.folder".parse() {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, "Failed to parse folder MIME type");
            bail!("Failed to parse folder MIME type: {}", e);
        }
    };

    let result = hub
        .files()
        .create(folder_metadata)
        .param("fields", "id, name")
        .add_scope(Scope::Full)
        .upload(std::io::empty(), folder_mime)
        .await;

    let (_, created) = match result {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, folder_name = name, "Failed to create folder on Google Drive");
            bail!("Failed to create folder '{}': {}", name, e);
        }
    };

    match created.id {
        Some(id) => {
            info!(folder_name = name, folder_id = %id, "Created Drive folder");
            Ok(id)
        }
        None => {
            error!(
                folder_name = name,
                "Google Drive created folder but returned no ID"
            );
            bail!("Google Drive created folder '{}' but returned no ID", name);
        }
    }
}

/// Upload a local file to a specific Google Drive folder using resumable upload.
pub async fn upload_file(hub: &DriveHub, folder_id: &str, file_path: &Path) -> anyhow::Result<()> {
    let file_name = match file_path.file_name() {
        Some(name) => match name.to_str() {
            Some(s) => s.to_string(),
            None => {
                error!(path = ?file_path, "File name is not valid UTF-8");
                bail!("File name is not valid UTF-8: {:?}", file_path);
            }
        },
        None => {
            error!(path = %file_path.display(), "Cannot determine file name from path");
            bail!(
                "Cannot determine file name from path: {}",
                file_path.display()
            );
        }
    };

    let file_size = match tokio::fs::metadata(file_path).await {
        Ok(m) => m.len(),
        Err(e) => {
            error!(error = %e, path = %file_path.display(), "Failed to stat file for upload");
            bail!("Failed to stat {}: {}", file_path.display(), e);
        }
    };

    info!(
        file_name = %file_name,
        file_size_bytes = file_size,
        folder_id = folder_id,
        "Starting resumable upload to Google Drive"
    );

    let file_metadata = DriveFile {
        name: Some(file_name.clone()),
        parents: Some(vec![folder_id.to_string()]),
        ..Default::default()
    };

    let raw_file = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            error!(error = %e, path = %file_path.display(), "Failed to open file for upload");
            bail!("Failed to open {}: {}", file_path.display(), e);
        }
    };
    let reader = BufReader::with_capacity(512 * 1024, raw_file);

    let mime_type: mime::Mime = match "application/octet-stream".parse() {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, "Failed to parse upload MIME type");
            bail!("Failed to parse upload MIME type: {}", e);
        }
    };

    let result = hub
        .files()
        .create(file_metadata)
        .param("fields", "id, name, size")
        .add_scope(Scope::Full)
        .upload_resumable(reader, mime_type)
        .await;

    match result {
        Ok((_, uploaded)) => {
            let id = match uploaded.id.as_deref() {
                Some(id) => id,
                None => "unknown",
            };
            info!(
                file_name = %file_name,
                drive_file_id = id,
                file_size_bytes = file_size,
                "Upload completed"
            );
            Ok(())
        }
        Err(e) => {
            error!(
                error = %e,
                file_name = %file_name,
                "Failed to upload file to Google Drive"
            );
            bail!("Failed to upload '{}' to Google Drive: {}", file_name, e);
        }
    }
}
