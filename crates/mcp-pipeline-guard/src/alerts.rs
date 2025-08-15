//! Alert management and notification system

use mcp_common::{Error, Result};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert channels for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannel {
    Log,
    Webhook { url: String },
    Email { address: String },
    Slack { webhook_url: String },
    Custom { handler_name: String },
}

/// Alert message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
    pub component_id: String,
    pub title: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

impl Alert {
    pub fn new(
        component_id: String,
        severity: AlertSeverity,
        title: String,
        message: String,
    ) -> Self {
        Alert {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            severity,
            component_id,
            title,
            message,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub channels: Vec<AlertChannel>,
    pub min_severity: AlertSeverity,
    pub rate_limit_seconds: u64,
    pub batch_size: usize,
    pub batch_timeout_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        AlertConfig {
            channels: vec![AlertChannel::Log],
            min_severity: AlertSeverity::Warning,
            rate_limit_seconds: 60,
            batch_size: 10,
            batch_timeout_seconds: 30,
        }
    }
}

/// Alert manager for handling notifications
pub struct AlertManager {
    config: AlertConfig,
    alert_sender: mpsc::UnboundedSender<Alert>,
    _alert_processor: tokio::task::JoinHandle<()>,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self::with_config(AlertConfig::default())
    }

    /// Create alert manager with custom configuration
    pub fn with_config(config: AlertConfig) -> Self {
        let (alert_sender, mut alert_receiver) = mpsc::unbounded_channel::<Alert>();
        
        let channels = config.channels.clone();
        let alert_processor = tokio::spawn(async move {
            let mut alert_batch = Vec::new();
            let mut last_batch_send = Utc::now();
            
            while let Some(alert) = alert_receiver.recv().await {
                alert_batch.push(alert);
                
                let should_send_batch = alert_batch.len() >= config.batch_size
                    || Utc::now()
                        .signed_duration_since(last_batch_send)
                        .num_seconds() >= config.batch_timeout_seconds as i64;
                
                if should_send_batch && !alert_batch.is_empty() {
                    if let Err(e) = Self::send_alert_batch(&channels, &alert_batch).await {
                        error!("Failed to send alert batch: {}", e);
                    }
                    alert_batch.clear();
                    last_batch_send = Utc::now();
                }
            }
            
            // Send any remaining alerts
            if !alert_batch.is_empty() {
                if let Err(e) = Self::send_alert_batch(&channels, &alert_batch).await {
                    error!("Failed to send final alert batch: {}", e);
                }
            }
        });

        AlertManager {
            config,
            alert_sender,
            _alert_processor: alert_processor,
        }
    }

    /// Send a component health alert
    pub async fn send_component_alert(&self, component_id: &str, reason: &str) -> Result<()> {
        let alert = Alert::new(
            component_id.to_string(),
            AlertSeverity::Critical,
            "Component Health Alert".to_string(),
            format!("Component {} is unhealthy: {}", component_id, reason),
        );

        self.send_alert(alert).await
    }

    /// Send a recovery success alert
    pub async fn send_recovery_success_alert(&self, component_id: &str) -> Result<()> {
        let alert = Alert::new(
            component_id.to_string(),
            AlertSeverity::Info,
            "Recovery Successful".to_string(),
            format!("Component {} has been successfully recovered", component_id),
        );

        self.send_alert(alert).await
    }

    /// Send a recovery failed alert
    pub async fn send_recovery_failed_alert(&self, component_id: &str, error: &str) -> Result<()> {
        let alert = Alert::new(
            component_id.to_string(),
            AlertSeverity::Emergency,
            "Recovery Failed".to_string(),
            format!("Failed to recover component {}: {}", component_id, error),
        );

        self.send_alert(alert).await
    }

    /// Send a custom alert
    pub async fn send_alert(&self, alert: Alert) -> Result<()> {
        if alert.severity < self.config.min_severity {
            debug!("Alert below minimum severity threshold, skipping: {:?}", alert);
            return Ok(());
        }

        debug!("Sending alert: {} - {}", alert.title, alert.message);
        
        self.alert_sender
            .send(alert)
            .map_err(|e| Error::Internal(format!("Failed to queue alert: {}", e)))?;

        Ok(())
    }

    /// Send a batch of alerts to all configured channels
    async fn send_alert_batch(channels: &[AlertChannel], alerts: &[Alert]) -> Result<()> {
        debug!("Sending batch of {} alerts", alerts.len());

        for channel in channels {
            if let Err(e) = Self::send_to_channel(channel, alerts).await {
                error!("Failed to send alerts to channel {:?}: {}", channel, e);
            }
        }

        Ok(())
    }

    /// Send alerts to a specific channel
    async fn send_to_channel(channel: &AlertChannel, alerts: &[Alert]) -> Result<()> {
        match channel {
            AlertChannel::Log => {
                for alert in alerts {
                    match alert.severity {
                        AlertSeverity::Info => info!("[ALERT] {}: {}", alert.title, alert.message),
                        AlertSeverity::Warning => warn!("[ALERT] {}: {}", alert.title, alert.message),
                        AlertSeverity::Critical | AlertSeverity::Emergency => {
                            error!("[ALERT] {}: {}", alert.title, alert.message)
                        }
                    }
                }
            }
            AlertChannel::Webhook { url } => {
                Self::send_webhook_alerts(url, alerts).await?;
            }
            AlertChannel::Email { address } => {
                Self::send_email_alerts(address, alerts).await?;
            }
            AlertChannel::Slack { webhook_url } => {
                Self::send_slack_alerts(webhook_url, alerts).await?;
            }
            AlertChannel::Custom { handler_name } => {
                Self::send_custom_alerts(handler_name, alerts).await?;
            }
        }

        Ok(())
    }

    /// Send alerts via webhook
    async fn send_webhook_alerts(url: &str, alerts: &[Alert]) -> Result<()> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "alerts": alerts,
            "timestamp": Utc::now(),
            "count": alerts.len()
        });

        let response = client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Webhook request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Webhook returned status: {}",
                response.status()
            )));
        }

        debug!("Successfully sent {} alerts via webhook", alerts.len());
        Ok(())
    }

    /// Send alerts via email (placeholder implementation)
    async fn send_email_alerts(_address: &str, alerts: &[Alert]) -> Result<()> {
        // In a real implementation, this would integrate with an email service
        debug!("Email alerts not implemented, logging {} alerts", alerts.len());
        for alert in alerts {
            info!("[EMAIL] {}: {}", alert.title, alert.message);
        }
        Ok(())
    }

    /// Send alerts via Slack (placeholder implementation)
    async fn send_slack_alerts(webhook_url: &str, alerts: &[Alert]) -> Result<()> {
        let client = reqwest::Client::new();
        
        for alert in alerts {
            let emoji = match alert.severity {
                AlertSeverity::Info => ":information_source:",
                AlertSeverity::Warning => ":warning:",
                AlertSeverity::Critical => ":exclamation:",
                AlertSeverity::Emergency => ":rotating_light:",
            };

            let payload = serde_json::json!({
                "text": format!("{} *{}*\n{}", emoji, alert.title, alert.message),
                "username": "Pipeline Guard",
                "icon_emoji": ":robot_face:"
            });

            let response = client
                .post(webhook_url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| Error::Internal(format!("Slack webhook failed: {}", e)))?;

            if !response.status().is_success() {
                return Err(Error::Internal(format!(
                    "Slack webhook returned status: {}",
                    response.status()
                )));
            }
        }

        debug!("Successfully sent {} alerts via Slack", alerts.len());
        Ok(())
    }

    /// Send alerts via custom handler (placeholder implementation)
    async fn send_custom_alerts(handler_name: &str, alerts: &[Alert]) -> Result<()> {
        // In a real implementation, this would call registered custom handlers
        debug!("Custom alert handler '{}' not implemented, logging {} alerts", 
               handler_name, alerts.len());
        for alert in alerts {
            info!("[CUSTOM:{}] {}: {}", handler_name, alert.title, alert.message);
        }
        Ok(())
    }

    /// Update alert configuration
    pub fn update_config(&mut self, config: AlertConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &AlertConfig {
        &self.config
    }
}

impl Drop for AlertManager {
    fn drop(&mut self) {
        self._alert_processor.abort();
    }
}