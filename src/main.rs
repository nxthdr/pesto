mod config;
mod producer;
mod serializer;
mod sflow;
mod sflow_capnp;

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::channel;
use tokio_graceful::Shutdown;
use tracing::{debug, error, trace};

use crate::config::{configure, AppConfig};

async fn sflow_handler(cfg: Arc<AppConfig>) {
    let sflow_config = cfg.sflow.clone();
    let kafka_config = cfg.kafka.clone();

    debug!("binding sFlow listener to {}", sflow_config.host);
    let socket = UdpSocket::bind(sflow_config.host)
        .await
        .expect("Failed to bind UDP socket");

    // Initialize MPSC channel to communicate between sFlow handler and producer
    let (tx, rx) = channel(kafka_config.mpsc_buffer_size);

    // Spawn producer task
    let producer_handle = tokio::spawn(async move {
        if let Err(err) = producer::handle(&kafka_config, rx).await {
            error!("Error handling Kafka producer: {}", err);
        }
    });

    // Handle sFlow datagrams
    let sflow_handle = tokio::spawn(async move {
        if let Err(err) = sflow::handle(socket, tx).await {
            error!("Error handling sFlow datagrams: {}", err);
        }
    });

    // Wait for both tasks
    tokio::select! {
        _ = sflow_handle => {}
        _ = producer_handle => {}
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Arc::new(configure().await?);
    trace!("{:?}", cfg);

    let shutdown = Shutdown::default();

    // Initialize sFlow handler task
    let sflow_task = shutdown.spawn_task(sflow_handler(cfg.clone()));

    tokio::select! {
        biased;
        _ = shutdown.shutdown_with_limit(Duration::from_secs(1)) => {}
        _ = sflow_task => {}
    }

    Ok(())
}
