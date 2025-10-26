use anyhow::Result;
use bytes::Bytes;
use chrono::Utc;
use metrics::counter;
use sflow_parser::{parse_datagram, SFlowDatagram};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, trace};

const MAX_DATAGRAM_SIZE: usize = 65535;

pub async fn handle(socket: UdpSocket, tx: Sender<(SFlowDatagram, i64, SocketAddr)>) -> Result<()> {
    let local_addr = socket.local_addr()?;
    debug!("sFlow listener bound to {}", local_addr);

    let mut buf = vec![0u8; MAX_DATAGRAM_SIZE];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n_bytes, peer_addr)) => {
                if n_bytes == 0 {
                    continue;
                }

                let time_received_ns = Utc::now().timestamp_nanos_opt().unwrap();
                trace!("Received {} bytes from {}", n_bytes, peer_addr);

                // Parse the sFlow datagram
                let data = Bytes::copy_from_slice(&buf[..n_bytes]);
                match parse_datagram(&data) {
                    Ok(datagram) => {
                        counter!("pesto_sflow_datagrams_total", "status" => "success").increment(1);
                        counter!("pesto_sflow_samples_total").increment(datagram.samples.len() as u64);
                        
                        trace!(
                            "Parsed sFlow datagram: version={:?}, agent={:?}, samples={}",
                            datagram.version,
                            datagram.agent_address,
                            datagram.samples.len()
                        );

                        // Send to producer
                        if let Err(e) = tx.send((datagram, time_received_ns, peer_addr)).await {
                            error!("Failed to send datagram to producer: {}", e);
                        }
                    }
                    Err(e) => {
                        counter!("pesto_sflow_datagrams_total", "status" => "parse_error").increment(1);
                        error!("Failed to parse sFlow datagram from {}: {:?}", peer_addr, e);
                        trace!("Failed datagram data: {:02x?}", &buf[..n_bytes]);
                    }
                }
            }
            Err(e) => {
                error!("Failed to receive UDP datagram: {}", e);
                return Err(e.into());
            }
        }
    }
}
