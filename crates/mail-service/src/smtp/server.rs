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
    let tls_bind_addr = config.smtp_tls_bind_addr.clone();
    let plain_service = Arc::clone(&service);
    let tls_service = Arc::clone(&service);

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

    // Spawn plain SMTP server task
    let plain_server_task = tokio::spawn(async move {
        loop {
            let result = tokio::task::spawn_blocking({
                let plain_addr = smtp_bind_addr.clone();
                let service = Arc::clone(&plain_service);
                move || -> Result<(), anyhow::Error> {
                    let handler = SmtpHandler::new(service);
                    let addr: SocketAddr = plain_addr.parse()?;
                    let mut server = Server::new(handler);
                    server
                        .with_name("plain")
                        .with_addr(addr)
                        .map_err(|e| anyhow::anyhow!("Failed to configure plain SMTP server: {}", e))?;
                    info!("Plain SMTP server listening on {}", addr);
                    server
                        .serve()
                        .map_err(|e| anyhow::anyhow!("Plain SMTP server error: {}", e))
                }
            }).await;
            match result {
                Ok(Ok(_)) => break,
                Ok(Err(e)) => {
                    warn!("Plain SMTP server error: {}", e);
                }
                Err(e) => {
                    warn!("Plain SMTP server panicked: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Spawn TLS SMTP server task if TLS configuration is provided
    let tls_server_task = tls_config.clone().map(|tls_config| tokio::spawn(async move {
        loop {
            let result = tokio::task::spawn_blocking({
                let tls_addr = tls_bind_addr.clone();
                let service = Arc::clone(&tls_service);
                let tls_config = tls_config.clone();
                move || -> Result<(), anyhow::Error> {
                    let handler = SmtpHandler::new(service);
                    let addr: SocketAddr = tls_addr.parse()?;
                    let mut server = Server::new(handler);
                    server
                        .with_name("tls")
                        .with_addr(addr)
                        .map_err(|e| anyhow::anyhow!("Failed to configure TLS SMTP server: {}", e))?;
                    info!("Configuring TLS for SMTP server");
                    server
                        .with_ssl(SslConfig::Trusted {
                            cert_path: tls_config.0.to_string_lossy().to_string(),
                            key_path: tls_config.1.to_string_lossy().to_string(),
                            chain_path: tls_config.2.to_string_lossy().to_string(),
                        })
                        .map_err(|e| anyhow::anyhow!("Failed to configure TLS: {}", e))?;
                    info!("TLS SMTP server listening on {}", addr);
                    server
                        .serve()
                        .map_err(|e| anyhow::anyhow!("TLS SMTP server error: {}", e))
                }
            }).await;
            match result {
                Ok(Ok(_)) => break,
                Ok(Err(e)) => {
                    warn!("TLS SMTP server error: {}", e);
                }
                Err(e) => {
                    warn!("TLS SMTP server panicked: {}", e);
                }
            }
            let changed = tokio::time::timeout(std::time::Duration::from_secs(5), rx.changed()).await;
            if changed.is_ok() {
                info!("TLS configuration changed, restarting TLS SMTP server");
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }));

    // Wait for the plain server and, if applicable, the TLS server tasks concurrently
    if let Some(tls_task) = tls_server_task {
        let _ = tokio::try_join!(plain_server_task, tls_task)?;
    } else {
        plain_server_task.await?;
    }

    Ok(())
}
