use crate::{config::Config, service::MailService, smtp::handler::SmtpHandler};
use anyhow::Result;
use mailin_embedded::{Server, SslConfig};
use notify::{Config as NotifyConfig, Event, PollWatcher, RecursiveMode, Watcher};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::watch, task};
use tracing::{info, warn};

pub async fn run_smtp_server(
    config: &Config,
    service: Arc<MailService>,
) -> Result<(), anyhow::Error> {
    // Clone the necessary values from config before moving into the task
    let smtp_bind_addr = config.smtp_bind_addr.clone();
    let tls_config = config
        .tls_cert_path
        .as_ref()
        .zip(config.tls_key_path.as_ref())
        .zip(config.tls_chain_path.as_ref())
        .map(|((cert, key), chain)| (cert.clone(), key.clone(), chain.clone()));

    // Set up file watching if TLS is configured
    let (tx, mut rx) = watch::channel(());

    if let Some((cert_path, key_path, chain_path)) = tls_config.clone() {
        let paths = vec![cert_path.clone(), key_path.clone(), chain_path.clone()];
        let tx = tx.clone();
        let poll_interval = Duration::from_secs(config.tls_poll_interval);

        task::spawn(async move {
            let config = NotifyConfig::default().with_poll_interval(poll_interval);

            let mut watcher = PollWatcher::new(
                move |res: Result<Event, notify::Error>| match res {
                    Ok(_) => {
                        if let Err(e) = tx.send(()) {
                            warn!("Failed to send restart signal: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Watch error: {}", e);
                    }
                },
                config,
            )
            .unwrap();

            for path in paths {
                if let Err(e) = watcher.watch(path.as_ref(), RecursiveMode::NonRecursive) {
                    warn!("Failed to watch TLS file: {}", e);
                }
            }

            // Keep the watcher alive
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }

    loop {
        let server_handle = task::spawn_blocking({
            let smtp_bind_addr = smtp_bind_addr.clone();
            let tls_config = tls_config.clone();
            let service = Arc::clone(&service);

            move || -> Result<(), anyhow::Error> {
                let handler = SmtpHandler::new(service);
                let addr: SocketAddr = smtp_bind_addr.parse()?;
                let mut server = Server::new(handler);

                server
                    .with_name("localhost")
                    .with_addr(addr)
                    .map_err(|e| anyhow::anyhow!("Failed to configure SMTP server: {}", e))?;

                // Configure TLS if certificates are provided
                if let Some((cert_path, key_path, chain_path)) = tls_config {
                    info!("Configuring TLS for SMTP server");
                    server
                        .with_ssl(SslConfig::Trusted {
                            cert_path: cert_path.to_string_lossy().to_string(),
                            key_path: key_path.to_string_lossy().to_string(),
                            chain_path: chain_path.to_string_lossy().to_string(),
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to configure TLS: {}", e))?;
                }

                info!("SMTP server listening on {}", addr);
                server
                    .serve()
                    .map_err(|e| anyhow::anyhow!("SMTP server error: {}", e))?;

                Ok(())
            }
        });

        tokio::select! {
            result = server_handle => {
                if let Err(e) = result {
                    warn!("Server task failed: {}", e);
                }
            }
            _ = rx.changed() => {
                info!("TLS files changed, restarting server...");
                continue;
            }
        }
    }
}
