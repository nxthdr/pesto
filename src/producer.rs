use anyhow::Result;
use metrics::counter;
use rdkafka::config::ClientConfig;
use rdkafka::message::OwnedHeaders;
use rdkafka::producer::{FutureProducer, FutureRecord};
use sflow_parser::SFlowDatagram;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, error, trace};

use crate::config::KafkaConfig;
use crate::serializer::serialize_sflow_record;

#[derive(Clone)]
pub struct SaslAuth {
    pub username: String,
    pub password: String,
    pub mechanism: String,
}

#[derive(Clone)]
pub enum KafkaAuth {
    SaslPlainText(SaslAuth),
    PlainText,
}

pub async fn handle(
    config: &KafkaConfig,
    mut rx: Receiver<(SFlowDatagram, i64, SocketAddr)>,
) -> Result<()> {
    // Configure Kafka authentication
    let kafka_auth = match config.auth_protocol.as_str() {
        "PLAINTEXT" => KafkaAuth::PlainText,
        "SASL_PLAINTEXT" => KafkaAuth::SaslPlainText(SaslAuth {
            username: config.auth_sasl_username.clone(),
            password: config.auth_sasl_password.clone(),
            mechanism: config.auth_sasl_mechanism.clone(),
        }),
        _ => {
            anyhow::bail!("invalid Kafka producer authentication protocol");
        }
    };

    if config.disable {
        debug!("producer disabled");
        while let Some(_datagram) = rx.recv().await {
            // Just consume and drop
        }
        return Ok(());
    }

    let kafka_brokers = config
        .brokers
        .iter()
        .map(|addr| addr.to_string())
        .collect::<Vec<_>>()
        .join(",");
    debug!(
        "Kafka producer enabled, connecting to brokers: {}",
        kafka_brokers
    );

    // Configure Kafka producer
    let mut client_config = ClientConfig::new();
    client_config
        .set("bootstrap.servers", kafka_brokers)
        .set("message.timeout.ms", config.message_timeout_ms.to_string());

    if let KafkaAuth::SaslPlainText(auth) = kafka_auth {
        client_config = client_config
            .set("sasl.username", auth.username)
            .set("sasl.password", auth.password)
            .set("sasl.mechanisms", auth.mechanism)
            .set("security.protocol", "SASL_PLAINTEXT")
            .to_owned();
    }

    let producer: FutureProducer = client_config
        .create()
        .expect("Failed to create Kafka producer");

    // Send to Kafka
    let mut additional_messages: Vec<Vec<u8>> = Vec::new();
    loop {
        let start_time = std::time::Instant::now();
        let mut final_message = Vec::new();
        let mut n_records = 0;

        // Send the additional messages first
        for message in additional_messages.drain(..) {
            final_message.extend_from_slice(&message);
            n_records += 1;
        }

        loop {
            if std::time::Instant::now().duration_since(start_time)
                > std::time::Duration::from_millis(config.batch_wait_time)
            {
                break;
            }

            let datagram = rx.try_recv();
            if datagram.is_err() {
                tokio::time::sleep(Duration::from_millis(config.batch_wait_interval)).await;
                continue;
            }

            let (datagram, time_received_ns, peer_addr) = datagram.unwrap();
            trace!("Processing datagram from {}", peer_addr);

            // Serialize the sFlow records
            let messages = serialize_sflow_record(&datagram, time_received_ns, peer_addr);

            for message in messages {
                // Max message size check
                if final_message.len() + message.len() > config.message_max_bytes {
                    additional_messages.push(message);
                    break;
                }

                final_message.extend_from_slice(&message);
                n_records += 1;
            }
        }

        if final_message.is_empty() {
            continue;
        }

        debug!("sending {} sFlow records to Kafka", n_records);
        let delivery_status = producer
            .send(
                FutureRecord::to(config.topic.as_str())
                    .payload(&final_message)
                    .key("")
                    .headers(OwnedHeaders::new()),
                Duration::from_secs(0),
            )
            .await;

        let metric_name = "pesto_kafka_messages_total";
        match delivery_status {
            Ok(delivery) => {
                counter!(metric_name, "status" => "success").increment(1);
                counter!("pesto_sflow_records_total").increment(n_records);
                debug!(
                    "successfully sent message to partition {} at offset {}",
                    delivery.partition, delivery.offset
                );
            }
            Err((error, _)) => {
                counter!(metric_name, "status" => "failure").increment(1);
                error!("failed to send message: {}", error);
            }
        }
    }
}
