use std::path::Path;

use anyhow::bail;
use tracing::{error, info};

pub type DriveHub = google_drive3::DriveHub<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
>;

pub async fn build_hub(credentials_path: &Path) -> anyhow::Result<DriveHub> {
    info!(
        path = %credentials_path.display(),
        "Authenticating with Google Drive"
    );

    // Install the rustls crypto provider before any TLS operations.
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        tracing::debug!(error = ?e, "CryptoProvider already installed, continuing");
    }

    let secret = match yup_oauth2::read_authorized_user_secret(credentials_path).await {
        Ok(s) => s,
        Err(e) => {
            error!(
                error = %e,
                path = %credentials_path.display(),
                "Failed to read authorized user credentials"
            );
            bail!(
                "Failed to read authorized user credentials from {}: {}",
                credentials_path.display(),
                e
            );
        }
    };

    let auth = match yup_oauth2::AuthorizedUserAuthenticator::builder(secret)
        .build()
        .await
    {
        Ok(a) => a,
        Err(e) => {
            error!(error = %e, "Failed to build authenticator");
            bail!("Failed to build authenticator: {}", e);
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
