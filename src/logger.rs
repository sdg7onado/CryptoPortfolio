use crate::errors::PortfolioError;
use env_logger::Builder;
use hex;
use hmac::{Hmac, Mac};
use log::{info, LevelFilter};
use sha2::Sha256;
use std::fs::OpenOptions;
use std::io::Write;

pub fn init_logger(env: &str) -> Result<(), PortfolioError> {
    let level = if env == "dev" {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    Builder::new()
        .filter_level(level)
        .try_init()
        .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;
    Ok(())
}

pub fn log_action(action: &str, env: &str) -> Result<(), PortfolioError> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let log = format!("[{}] {}\n", timestamp, action);
    info!("{}", action);
    if env == "prod" {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("portfolio_log.txt")
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;
        file.write_all(log.as_bytes())
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;
        // Optionally, sign logs for integrity
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret_key")
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;
        mac.update(log.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let signed_log = format!("{} [Signature: {}]\n", log.trim(), signature);
        file.write_all(signed_log.as_bytes())
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;
    }
    Ok(())
}
