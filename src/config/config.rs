pub struct Config {
    pub db_url: String,
    pub db_host: String,
    pub db_username: String,
    pub db_password: String,
    pub db_name: String,
    pub db_port: u16,
    pub google_service_account_path: String,
    pub google_drive_folder_id: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv()?;
        Ok(Config {
            db_url: std::env::var("DB_URL")?,
            db_host: std::env::var("DB_HOST")?,
            db_username: std::env::var("DB_USERNAME")?,
            db_password: std::env::var("DB_PASSWORD")?,
            db_name: std::env::var("DB_NAME")?,
            db_port: std::env::var("DB_PORT")?.parse()?,
            google_service_account_path: std::env::var("GOOGLE_SERVICE_ACCOUNT_PATH")?,
            google_drive_folder_id: std::env::var("GOOGLE_DRIVE_FOLDER_ID")?,
        })
    }
}
