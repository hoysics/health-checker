use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::error::Error;
use std::result::Result;

pub struct EmailSender {
    smtp_username: String,
    smtp_password: String,
    smtp_server: String,
    smtp_port: u16,
}

impl EmailSender {
    pub fn new(
        smtp_username: String,
        smtp_password: String,
        smtp_server: String,
        smtp_port: u16,
    ) -> Self {
        EmailSender {
            smtp_username,
            smtp_password,
            smtp_server,
            smtp_port,
        }
    }

    pub fn send_email(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), Box<dyn Error>> {
        let email = Message::builder()
            .from(from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .body(body.into())?;

        let credentials = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());
        let mailer = SmtpTransport::relay(&self.smtp_server)
            .unwrap()
            .credentials(credentials)
            .build();

        let result = mailer.send(&email)?;
        if result.is_positive() {
            Ok(())
        } else {
            Err("Failed to send email".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_email() {
        let email_sender = EmailSender::new(
            "your_smtp_username".to_string(),
            "your_smtp_password".to_string(),
            "smtp.example.com".to_string(),
            587,
        );

        let result = email_sender.send_email(
            "from@example.com",
            "to@example.com",
            "Test Subject",
            "Hello, this is a test email.",
        );

        assert!(result.is_ok());
    }
}
