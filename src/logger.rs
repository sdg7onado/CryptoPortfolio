use log::{error, info, LevelFilter};
use std::fs::OpenOptions;
use std::io::Write;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn init_logger(env: &str) {
    let level = if env == "dev" { LevelFilter::Debug } else { LevelFilter::Info };
    env_logger::Builder::new()
        .filter_level(level)
        .init();
}

pub fn log_action(action: &str, env: &str) {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let log = format!("[{}] {}\n", timestamp, action);
    info!("{}", action);
    if env == "prod" {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("portfolio_log.txt")
            .expect("Failed to open log file");
        file.write_all(log.as_bytes()).expect("Failed to write to log");
        // Optionally, sign logs for integrity (simplified example)
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret_key").expect("HMAC error");
        mac.update(log.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let signed_log = format!("{} [Signature: {}]\n", log.trim(), signature);
        file.write_all(signed_log.as_bytes()).expect("Failed to write signed log");
    }
}