use {
    async_trait::async_trait,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub data: Option<NotificationData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub download_id: Option<u64>,
    pub download_path: Option<PathBuf>,
    pub backend: Option<String>,
}

#[async_trait::async_trait]
pub trait NotificationBackend: Send + Sync {
    fn source(&self) -> &'static str;
    async fn send(&self, notification: &Notification) -> anyhow::Result<()>;
}

pub struct SystemNotifier;

#[async_trait::async_trait]
impl NotificationBackend for SystemNotifier {
    fn source(&self) -> &'static str { "system" }

    async fn send(&self, notification: &Notification) -> anyhow::Result<()> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let body = notification.body.replace('"', "\\\"");
            let title = notification.title.replace('"', "\\\"");
            let _ = Command::new("notify-send")
                .args(&["-a", "bldhnd", "-i", notification.icon.as_deref().unwrap_or("dialog-information"), &title, &body])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let body = notification.body.replace('"', "\\\"");
            let title = notification.title.replace('"', "\\\"");
            let _ = Command::new("osascript")
                .args(&["-e", &format!("display notification \"{body}\" with title \"{title}\"")])
                .spawn();
        }
        Ok(())
    }
}

pub struct WebhookNotifier {
    url: String,
}

impl WebhookNotifier {
    pub fn new(url: &str) -> Self { Self { url: url.to_string() } }
}

#[async_trait::async_trait]
impl NotificationBackend for WebhookNotifier {
    fn source(&self) -> &'static str { "webhook" }

    async fn send(&self, notification: &Notification) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client.post(&self.url).json(notification).send().await?;
        Ok(())
    }
}
