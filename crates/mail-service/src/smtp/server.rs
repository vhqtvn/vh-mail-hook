use crate::{config::Config, service::MailService, smtp::handler::SmtpHandler};
use anyhow::Result;
use mailin_embedded::{Server, SslConfig};
use std::{net::SocketAddr, sync::Arc};
use tokio::task;
use tracing::info;

pub async fn run_smtp_server(
    config: &Config,
    service: Arc<MailService>,
) -> Result<(), anyhow::Error> {
    // Clone the necessary values from config before moving into the task
    let smtp_bind_addr = config.smtp_bind_addr.clone();
    let email_domain = config.email_domain.clone();
    let tls_config = config.tls_cert_path.as_ref().zip(config.tls_key_path.as_ref()).zip(config.tls_chain_path.as_ref())
        .map(|((cert, key), chain)| (cert.clone(), key.clone(), chain.clone()));

    task::spawn_blocking(move || {
        let handler = SmtpHandler::new(service);

        let addr: SocketAddr = smtp_bind_addr.parse()?;
        let mut server = Server::new(handler);

        server
            .with_name(email_domain)
            .with_addr(addr)
            .map_err(|e| anyhow::anyhow!("Failed to configure SMTP server: {}", e))?;

        // Configure TLS if certificates are provided
        if let Some((cert_path, key_path, chain_path)) = tls_config {
            info!("Configuring TLS for SMTP server");
            server.with_ssl(SslConfig::Trusted {
                cert_path: cert_path.to_string_lossy().to_string(),
                key_path: key_path.to_string_lossy().to_string(),
                chain_path: chain_path.to_string_lossy().to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to configure TLS: {}", e))?;
        }

        info!("SMTP server listening on {}", addr);
        server.serve()
            .map_err(|e| anyhow::anyhow!("SMTP server error: {}", e))?;

        Ok(())
    })
    .await?
}
