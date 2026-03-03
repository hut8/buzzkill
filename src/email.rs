use std::sync::mpsc;
use std::thread;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::output::format_mac;

pub struct DroneAlert {
    pub transport: &'static str,
    pub mac: [u8; 6],
    pub rssi: i8,
}

pub struct SmtpConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub from: lettre::Address,
    pub to: lettre::Address,
}

impl SmtpConfig {
    /// Read and validate all SMTP env vars. Returns None if SMTP_HOST is unset,
    /// or Err if config is partially set / invalid.
    pub fn from_env() -> Result<Option<Self>, String> {
        let host = match std::env::var("SMTP_HOST") {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let username = std::env::var("SMTP_USERNAME")
            .map_err(|_| "SMTP_HOST is set but SMTP_USERNAME is missing")?;
        let password = std::env::var("SMTP_PASSWORD")
            .map_err(|_| "SMTP_HOST is set but SMTP_PASSWORD is missing")?;
        let from: lettre::Address = std::env::var("SMTP_FROM")
            .map_err(|_| "SMTP_HOST is set but SMTP_FROM is missing")?
            .parse()
            .map_err(|e| format!("invalid SMTP_FROM address: {}", e))?;
        let to: lettre::Address = std::env::var("NOTIFY_TO")
            .map_err(|_| "SMTP_HOST is set but NOTIFY_TO is missing")?
            .parse()
            .map_err(|e| format!("invalid NOTIFY_TO address: {}", e))?;

        Ok(Some(Self {
            host,
            username,
            password,
            from,
            to,
        }))
    }
}

pub fn spawn_notifier(config: SmtpConfig, rx: mpsc::Receiver<DroneAlert>) {
    thread::spawn(move || {
        let creds = Credentials::new(config.username, config.password);
        let mailer = match SmtpTransport::relay(&config.host) {
            Ok(builder) => builder.credentials(creds).build(),
            Err(e) => {
                log::error!("Failed to create SMTP transport: {}", e);
                return;
            }
        };

        log::info!("Email notifier ready (to={})", config.to);

        while let Ok(alert) = rx.recv() {
            let mac_str = format_mac(&alert.mac);
            let subject = format!("Buzzkill: drone detected [{}]", mac_str);
            let body = format!(
                "A drone has been detected.\n\n\
                 Transport: {}\n\
                 MAC: {}\n\
                 RSSI: {} dBm\n",
                alert.transport, mac_str, alert.rssi,
            );

            let email = match Message::builder()
                .from(lettre::message::Mailbox::new(None, config.from.clone()))
                .to(lettre::message::Mailbox::new(None, config.to.clone()))
                .subject(&subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body)
            {
                Ok(e) => e,
                Err(e) => {
                    log::error!("Failed to build email: {}", e);
                    continue;
                }
            };

            match mailer.send(&email) {
                Ok(_) => log::info!("Notification sent for drone {}", mac_str),
                Err(e) => log::error!("Failed to send notification: {}", e),
            }
        }

        log::info!("Email notifier shutting down");
    });
}
