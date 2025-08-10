use crate::config::NotificationConfig;
use crate::errors::PortfolioError;
use crate::portfolio::Portfolio;
use reqwest::Client;
use std::collections::HashMap;

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

    pub async fn notify_significant_action(&self, action: &str) -> Result<(), PortfolioError> {
        if self.config.sms_enabled {
            self.send_sms(action).await?;
        }
        if self.config.email_enabled {
            self.send_email("Portfolio Action", action).await?;
        }
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
        let value_change_percent =
            ((current_value - previous_value) / previous_value.abs()) * 100.0;
        if value_change_percent.abs()
            > self
                .config
                .notification_thresholds
                .portfolio_value_change_percent
        {
            let msg = format!(
                "Portfolio value changed by {:.2}%: Previous ${:.2}, Current ${:.2}",
                value_change_percent, previous_value, current_value
            );
            if self.config.sms_enabled {
                self.send_sms(&msg).await?;
            }
            if self.config.email_enabled {
                self.send_email("Portfolio Value Change Alert", &msg)
                    .await?;
            }
        }

        for holding in &portfolio.holdings {
            if let (Some(prev_price), Some(curr_price)) = (
                previous_prices.get(&holding.symbol),
                current_prices.get(&holding.symbol),
            ) {
                let price_change_percent = ((curr_price - prev_price) / prev_price.abs()) * 100.0;
                if price_change_percent.abs()
                    > self
                        .config
                        .notification_thresholds
                        .holding_value_change_percent
                {
                    let msg = format!(
                        "{} price changed by {:.2}%: Previous ${:.2}, Current ${:.2}",
                        holding.symbol, price_change_percent, prev_price, curr_price
                    );
                    if self.config.sms_enabled {
                        self.send_sms(&msg).await?;
                    }
                    if self.config.email_enabled {
                        self.send_email("Holding Price Change Alert", &msg).await?;
                    }
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
            let msg = format!(
                "{} sentiment changed by {:.2}: Previous {:.2}, Current {:.2}",
                symbol, sentiment_change, previous_sentiment, current_sentiment
            );
            if self.config.sms_enabled {
                self.send_sms(&msg).await?;
            }
            if self.config.email_enabled {
                self.send_email("Sentiment Change Alert", &msg).await?;
            }
        }
        Ok(())
    }

    async fn send_sms(&self, message: &str) -> Result<(), PortfolioError> {
        let truncated_message = message[0..message.len().min(115)].to_string(); // Convert to String
                                                                                // let response = self
                                                                                //     .client
                                                                                //     .post("https://api.twilio.com/2010-04-01/Accounts")
                                                                                //     .basic_auth(
                                                                                //         &self.config.twilio_account_sid,
                                                                                //         Some(&self.config.twilio_auth_token),
                                                                                //     )
                                                                                //     .form(&[
                                                                                //         ("From", &self.config.twilio_phone_number),
                                                                                //         ("To", &self.config.recipient_phone_number),
                                                                                //         ("Body", &truncated_message), // Use String
                                                                                //     ])
                                                                                //     .send()
                                                                                //     .await
                                                                                //     .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;

        // if !response.status().is_success() {
        //     return Err(PortfolioError::NotificationError(format!(
        //         "SMS failed: {}",
        //         response.text().await.unwrap_or_default()
        //     )));
        // }
        Ok(())
    }

    async fn send_email(&self, subject: &str, body: &str) -> Result<(), PortfolioError> {
        let email = serde_json::json!({
            "personalizations": [{
                "to": [{"email": &self.config.recipient_email}]
            }],
            "from": {"email": &self.config.sender_email},
            "subject": subject,
            "content": [{
                "type": "text/html",
                "value": format!("<h2>{}</h2><p>{}</p><p><strong>Timestamp:</strong> {}</p>", subject, body, chrono::Utc::now())
            }]
        });

        // let response = self
        //     .client
        //     .post("https://api.sendgrid.com/v3/mail/send")
        //     .bearer_auth(&self.config.sendgrid_api_key)
        //     .json(&email)
        //     .send()
        //     .await
        //     .map_err(|e| PortfolioError::NotificationError(e.to_string()))?;

        // if !response.status().is_success() {
        //     return Err(PortfolioError::NotificationError(format!(
        //         "Email failed: {}",
        //         response.text().await.unwrap_or_default()
        //     )));
        // }
        Ok(())
    }
}
