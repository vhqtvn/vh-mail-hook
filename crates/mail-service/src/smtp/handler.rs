use crate::service::MailService;
use mailin_embedded::{Handler, Response};
use std::sync::Mutex;
use std::{io, net::IpAddr, sync::Arc};
use tokio::runtime::Runtime;
use tracing::{debug, error, warn};

#[derive(Clone)]
pub struct SmtpHandler {
    service: Arc<MailService>,
    current_mail: Vec<u8>,
    recipients: Vec<String>,
    current_sender: Option<String>,
    client_ip: IpAddr,
    runtime: Arc<Mutex<Runtime>>,
}

impl SmtpHandler {
    pub fn new(service: Arc<MailService>) -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime for SMTP handler");

        Self {
            service,
            current_mail: Vec::new(),
            recipients: Vec::new(),
            current_sender: None,
            client_ip: "0.0.0.0".parse().unwrap(),
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }
}

#[async_trait::async_trait]
impl Handler for SmtpHandler {
    fn helo(&mut self, client_ip: IpAddr, _domain: &str) -> Response {
        self.client_ip = client_ip;
        // Check if IP is blocked
        if self.service.is_ip_blocked(self.client_ip) {
            warn!("Blocked connection from IP: {}", self.client_ip);
            return Response::custom(250, "OK".to_string());
        }

        // Check rate limit
        if !self.service.check_rate_limit(self.client_ip) {
            warn!("Rate limit exceeded for IP: {}", self.client_ip);
            return Response::custom(250, "OK".to_string());
        }

        Response::custom(250, "OK".to_string())
    }

    fn mail(&mut self, _client_ip: IpAddr, from: &str, _parameters: &str) -> Response {
        self.current_mail.clear();
        self.recipients.clear();
        self.current_sender = Some(from.to_string());
        Response::custom(250, "Sender OK".to_string())
    }

    fn rcpt(&mut self, to: &str) -> Response {
        // Extract email from RCPT TO:<email@domain>
        let email = to.trim_start_matches("TO:<").trim_end_matches('>');
        self.recipients.push(email.to_string());
        Response::custom(250, "Recipient OK".to_string())
    }

    fn data_start(
        &mut self,
        _from: &str,
        _to: &str,
        _is_last: bool,
        _accepted: &[String],
    ) -> Response {
        if self.recipients.is_empty() {
            warn!("Attempted to send email with no valid recipients");
            return Response::custom(354, "Start mail input; end with <CRLF>.<CRLF>".to_string());
        }
        Response::custom(354, "Start mail input; end with <CRLF>.<CRLF>".to_string())
    }

    fn data(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.current_mail.len() + buf.len() > self.service.max_email_size() {
            warn!("Message size exceeds limit");
            // Still accept the data but truncate it
            self.current_mail.extend_from_slice(&buf[..self.service.max_email_size() - self.current_mail.len()]);
            return Ok(());
        }

        self.current_mail.extend_from_slice(buf);
        Ok(())
    }

    fn data_end(&mut self) -> Response {
        let mail_data = std::mem::take(&mut self.current_mail);
        let recipients = std::mem::take(&mut self.recipients);
        let service = self.service.clone();
        let sender = self.current_sender.clone().unwrap_or_default();
        let client_ip = self.client_ip;

        // Use the shared runtime to process the email
        match self.runtime.lock() {
            Ok(rt) => {
                // Process emails synchronously
                let results = rt.block_on(async {
                    let mut results = Vec::new();
                    for recipient in recipients {
                        let result = service
                            .process_incoming_email(&mail_data, &recipient, &sender, client_ip)
                            .await;
                        results.push((recipient, result));
                    }
                    results
                });

                // Log errors but don't expose them to sender
                for (recipient, result) in results {
                    match result {
                        Ok(_) => {
                            debug!("Email processed successfully for {}", recipient);
                        }
                        Err(e) => {
                            error!("Failed to process email for {}: {}", recipient, e);
                        }
                    }
                }

                // Always return success to sender
                Response::custom(250, "OK".to_string())
            }
            Err(e) => {
                error!("Failed to acquire runtime lock for email processing: {}", e);
                // Still return success to sender
                Response::custom(250, "OK".to_string())
            }
        }
    }
}
