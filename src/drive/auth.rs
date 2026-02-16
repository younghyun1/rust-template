use std::path::Path;

use anyhow::bail;
use tracing::{error, info};

pub type DriveHub = google_drive3::DriveHub<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
>;

pub async fn build_hub(service_account_path: &Path) -> anyhow::Result<DriveHub> {
    info!(
        path = %service_account_path.display(),
        "Authenticating with Google Drive service account"
    );

    // Install the rustls crypto provider before any TLS operations.
    // ring is pulled in by hyper-rustls; rustls requires explicit installation.
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        // Already installed is fine - log and continue
        tracing::debug!(error = ?e, "CryptoProvider already installed, continuing");
    }

    let sa_key_bytes = match tokio::fs::read(service_account_path).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(
                error = %e,
                path = %service_account_path.display(),
                "Failed to read service account key file"
            );
            bail!(
                "Failed to read service account key file {}: {}",
                service_account_path.display(),
                e
            );
        }
    };

    let sa_key: yup_oauth2::ServiceAccountKey = match serde_json::from_slice(&sa_key_bytes) {
        Ok(key) => key,
        Err(e) => {
            error!(
                error = %e,
                path = %service_account_path.display(),
                "Failed to parse service account key JSON"
            );
            bail!(
                "Failed to parse service account key from {}: {}",
                service_account_path.display(),
                e
            );
        }
    };

    let auth = match yup_oauth2::ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
    {
        Ok(a) => a,
        Err(e) => {
            error!(error = %e, "Failed to build service account authenticator");
            bail!("Failed to build service account authenticator: {}", e);
        }
    };

    let connector = match hyper_rustls::HttpsConnectorBuilder::new().with_native_roots() {
        Ok(builder) => builder.https_only().enable_http2().build(),
        Err(e) => {
            error!(error = %e, "Failed to load native TLS root certificates");
            bail!("Failed to load native TLS root certificates: {}", e);
        }
    };

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector);

    let hub = google_drive3::DriveHub::new(client, auth);

    info!("Google Drive authentication successful");

    Ok(hub)
}
