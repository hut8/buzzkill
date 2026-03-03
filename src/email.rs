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

pub fn spawn_notifier(rx: mpsc::Receiver<DroneAlert>) {
    thread::spawn(move || {
        let host = std::env::var("SMTP_HOST").expect("SMTP_HOST required");
        let username = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME required");
        let password = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD required");
        let from: lettre::Address = std::env::var("SMTP_FROM")
            .expect("SMTP_FROM required")
            .parse()
            .expect("invalid SMTP_FROM address");
        let to: lettre::Address = std::env::var("NOTIFY_TO")
            .expect("NOTIFY_TO required")
            .parse()
            .expect("invalid NOTIFY_TO address");

        let creds = Credentials::new(username, password);
        let mailer = SmtpTransport::relay(&host)
            .expect("failed to create SMTP transport")
            .credentials(creds)
            .build();

        log::info!("Email notifier ready (to={})", to);

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
                .from(lettre::message::Mailbox::new(None, from.clone()))
                .to(lettre::message::Mailbox::new(None, to.clone()))
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
