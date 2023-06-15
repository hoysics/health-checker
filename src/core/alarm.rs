use super::ent;
use chrono::Utc;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::Serialize;

#[derive(Serialize)]
struct EmailBody {
    update_time: String,
    events: Vec<ent::HealthInfo>,
}

pub struct Alarm {
    from: String,
    to: String,
    mailer: SmtpTransport,
}
impl Alarm {
    pub fn new(
        from: String,
        to: String,
        smtp_username: String,
        smtp_password: String,
        domain: String,
    ) -> Alarm {
        let creds = Credentials::new(smtp_username, smtp_password);
        Alarm {
            from,
            to,
            mailer: SmtpTransport::starttls_relay(&domain)
                .unwrap()
                .port(587)
                .credentials(creds)
                .build(),
        }
    }
    pub fn notify(&self, events: Vec<ent::HealthInfo>) {
        let body = EmailBody {
            update_time: Utc::now().to_rfc3339(),
            events,
        };
        let msg = serde_json::to_string(&body).unwrap();
        let email = Message::builder()
            .from(self.from.parse().unwrap())
            .to(self.to.parse().unwrap())
            .subject("资源监控预警")
            .header(ContentType::TEXT_PLAIN)
            .body(String::from(&msg))
            .unwrap();
        // Send the email
        match self.mailer.send(&email) {
            Ok(_) => tracing::info!("Email sent successfully!"),
            Err(e) => {
                tracing::error!("Could not send email: {e:?}");
                tracing::info!("Unsend mail: {:?}", &msg);
            }
        };
    }
}
#[cfg(test)]
mod tests {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    #[test]
    fn send_mail() {
        let email = Message::builder()
            .from("NoBody <nobody@domain.tld>".parse().unwrap())
            .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
            .to("Hei <hei@domain.tld>".parse().unwrap())
            .subject("Happy new year")
            .header(ContentType::TEXT_PLAIN)
            .body(String::from("Be happy!"))
            .unwrap();

        let creds = Credentials::new("smtp_username".to_owned(), "smtp_password".to_owned());

        // Open a remote connection to gmail
        let mailer = SmtpTransport::relay("smtp.gmail.com")
            .unwrap()
            .credentials(creds)
            .build();

        // Send the email
        match mailer.send(&email) {
            Ok(_) => println!("Email sent successfully!"),
            Err(e) => panic!("Could not send email: {e:?}"),
        };
    }
}
