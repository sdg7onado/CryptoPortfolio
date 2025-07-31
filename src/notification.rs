use reqwest::Client;
use serde::Deserialize;
use crate::errors::PortfolioError;
use crate::portfolio::Portfolio;
use std::collections::HashMap;
use chrono::Utc;

#[derive(Deserialize, Debug)]
pub struct NotificationConfig {
    pub sms_enabled: bool,
    pub email_enabled: bool,
    pub twilio_account_sid: String,
    pub twilio_auth_token: String,
    pub twilio_phone_number: String,
    pub recipient_phone_number: String,
    pub sendgrid_api_key: String,
    pub sender_email: String,
    pub recipient_email: String,
    pub notification_thresholds: NotificationThresholds,
}

#[derive(Deserialize, Debug)]
pub struct NotificationThresholds {
    pub portfolio_value_change_percent: f64, // e.g., 10.0 for 10%
    pub holding_value_change_percent: f64,  // e.g., 15.0 for 15%
    pub sentiment_change: f64,              // e.g., 0.2 for 20%
}

pub struct Notifier {
    client: Client,
    config: NotificationConfig,
}

impl Notifier {
    pub fn new(config: NotificationConfig) -> Self {
        Notifier {
            client: Client::new(),
            config,
        }
    }

    pub async fn send_sms(&self, message: &str) -> Result<(), PortfolioError> {
        if !self.config.sms_enabled {
            return Ok(());
        }
        let url = "https://api.twilio.com/2010-04-01/Accounts/".to_string() + &self.config.twilio_account_sid + "/Messages.json";
        let mut params = HashMap::new();
        params.insert("To", self.config.recipient_phone_number.clone());
        params.insert("From", self.config.twilio_phone_number.clone());
        params.insert("Body", message.to_string());

        let response = self.client
            .post(&url)
            .basic_auth(&self.config.twilio_account_sid, Some(&self.config.twilio_auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(PortfolioError::NotificationError(format!("SMS failed: {}", response.text().await.unwrap_or_default())))
        }
    }

    pub async fn send_email(&self, subject: &str, body: &str) -> Result<(), PortfolioError> {
        if !self.config.email_enabled {
            return Ok(());
        }
        let url = "https://api.sendgrid.com/v3/mail/send";
        let payload = serde_json::json!({
            "personalizations": [{
                "to": [{"email": self.config.recipient_email}]
            }],
            "from": {"email": self.config.sender_email},
            "subject": subject,
            "content": [{"type": "text/html", "value": body}]
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.sendgrid_api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(PortfolioError::NotificationError(format!("Email failed: {}", response.text().await.unwrap_or_default())))
        }
    }

    pub async fn notify_significant_action(&self, action: &str) -> Result<(), PortfolioError> {
        let sms_message = format!("Portfolio Action: {}. {}", action, Utc::now().to_rfc3339());
        let email_body = format!(
            "<h2>Portfolio Action Alert</h2><p><strong>Action:</strong> {}</p><p><strong>Timestamp:</strong> {}</p>",
            action, Utc::now().to_rfc3339()
        );
        self.send_sms(&sms_message).await?;
        self.send_email("Portfolio Action Alert", &email_body).await?;
        Ok(())
    }

    pub async fn notify_major_change(
        &self,
        portfolio: &Portfolio,
        previous_value: f64,
        current_value: f64,
        previous_prices: &HashMap<String, f64>,
        current_prices: &HashMap<String, f64>,
    ) -> Result<(), PortfolioError> {
        let value_change_percent = ((current_value - previous_value) / previous_value.abs()) * 100.0;
        if value_change_percent.abs() > self.config.notification_thresholds.portfolio_value_change_percent {
            let sms_message = format!(
                "Portfolio value changed by {:.2}%: ${:.2} to ${:.2}",
                value_change_percent, previous_value, current_value
            );
            let email_body = format!(
                "<h2>Portfolio Value Change Alert</h2><p><strong>Change:</strong> {:.2}%</p><p><strong>Previous:</strong> ${:.2}</p><p><strong>Current:</strong> ${:.2}</p><p><strong>Timestamp:</strong> {}</p>",
                value_change_percent, previous_value, current_value, Utc::now().to_rfc3339()
            );
            self.send_sms(&sms_message).await?;
            self.send_email("Portfolio Value Change Alert", &email_body).await?;
        }

        for holding in &portfolio.holdings {
            if let (Some(prev_price), Some(curr_price)) = (
                previous_prices.get(&holding.symbol),
                current_prices.get(&holding.symbol),
            ) {
                let change_percent = ((curr_price - prev_price) / prev_price.abs()) * 100.0;
                if change_percent.abs() > self.config.notification_thresholds.holding_value_change_percent {
                    let sms_message = format!(
                        "{} price changed by {:.2}%: ${:.2} to ${:.2}",
                        holding.symbol, change_percent, prev_price, curr_price
                    );
                    let email_body = format!(
                        "<h2>{} Price Change Alert</h2><p><strong>Change:</strong> {:.2}%</p><p><strong>Previous:</strong> ${:.2}</p><p><strong>Current:</strong> ${:.2}</p><p><strong>Timestamp:</strong> {}</p>",
                        holding.symbol, change_percent, prev_price, curr_price, Utc::now().to_rfc3339()
                    );
                    self.send_sms(&sms_message).await?;
                    self.send_email(&format!("{} Price Change Alert", holding.symbol), &email_body).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn notify_sentiment_change(
        &self,
        symbol: &str,
        previous_sentiment: f64,
        current_sentiment: f64,
    ) -> Result<(), PortfolioError> {
        let sentiment_change = current_sentiment - previous_sentiment;
        if sentiment_change.abs() > self.config.notification_thresholds.sentiment_change {
            let sms_message = format!(
                "{} sentiment changed by {:.2}: {:.2} to {:.2}",
                symbol, sentiment_change, previous_sentiment, current_sentiment
            );
            let email_body = format!(
                "<h2>{} Sentiment Change Alert</h2><p><strong>Change:</strong> {:.2}</p><p><strong>Previous:</strong> {:.2}</p><p><strong>Current:</strong> {:.2}</p><p><strong>Timestamp:</strong> {}</p>",
                symbol, sentiment_change, previous_sentiment, current_sentiment, Utc::now().to_rfc3339()
            );
            self.send_sms(&sms_message).await?;
            self.send_email(&format!("{} Sentiment Change Alert", symbol), &email_body).await?;
        }
        Ok(())
    }
}