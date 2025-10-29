use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tokio::net::lookup_host;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sflow: SFlowConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Clone)]
pub struct SFlowConfig {
    pub host: SocketAddr,
}

#[derive(Debug, Clone)]
pub struct KafkaConfig {
    pub disable: bool,
    pub brokers: Vec<SocketAddr>,
    pub topic: String,
    pub auth_protocol: String,
    pub auth_sasl_username: String,
    pub auth_sasl_password: String,
    pub auth_sasl_mechanism: String,
    pub message_max_bytes: usize,
    pub message_timeout_ms: usize,
    pub batch_wait_time: u64,
    pub batch_wait_interval: u64,
    pub mpsc_buffer_size: usize,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Pesto sFlow v5 collector", long_about = None)]
pub struct Cli {
    /// sFlow listener address (IP or FQDN)
    #[arg(long, default_value = "0.0.0.0:6343")]
    pub sflow_address: String,

    /// Kafka brokers (comma-separated list of address:port)
    #[arg(long, value_delimiter(','), default_value = "localhost:9092")]
    pub kafka_brokers: Vec<String>,

    /// Disable Kafka producer
    #[arg(long)]
    pub kafka_disable: bool,

    /// Kafka producer topic
    #[arg(long, default_value = "pesto-sflow")]
    pub kafka_topic: String,

    /// Kafka Authentication Protocol (e.g., PLAINTEXT, SASL_PLAINTEXT)
    #[arg(long, default_value = "PLAINTEXT")]
    pub kafka_auth_protocol: String,

    /// Kafka Authentication SASL Username
    #[arg(long, default_value = "pesto")]
    pub kafka_auth_sasl_username: String,

    /// Kafka Authentication SASL Password
    #[arg(long, default_value = "pesto")]
    pub kafka_auth_sasl_password: String,

    /// Kafka Authentication SASL Mechanism (e.g., PLAIN, SCRAM-SHA-512)
    #[arg(long, default_value = "SCRAM-SHA-512")]
    pub kafka_auth_sasl_mechanism: String,

    /// Kafka message max bytes
    #[arg(long, default_value_t = 990000)]
    pub kafka_message_max_bytes: usize,

    /// Kafka producer message timeout (ms)
    #[arg(long, default_value_t = 500000)]
    pub kafka_message_timeout_ms: usize,

    /// Kafka producer batch wait time (ms)
    #[arg(long, default_value_t = 1000)]
    pub kafka_batch_wait_time: u64,

    /// Kafka producer batch wait interval (ms)
    #[arg(long, default_value_t = 100)]
    pub kafka_batch_wait_interval: u64,

    /// Kafka MPSC buffer size
    #[arg(long, default_value_t = 100000)]
    pub kafka_mpsc_buffer_size: usize,

    /// Metrics listener address (IP or FQDN) for Prometheus endpoint
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub metrics_address: String,

    /// Set the verbosity level
    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

fn set_logging(cli: &Cli) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_max_level(cli.verbose)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn set_metrics(metrics_address: SocketAddr) {
    let prom_builder = PrometheusBuilder::new();
    prom_builder
        .with_http_listener(metrics_address)
        .install()
        .expect("Failed to install Prometheus metrics exporter");

    // Producer metrics
    metrics::describe_counter!(
        "pesto_kafka_messages_total",
        "Total number of Kafka messages produced"
    );

    // Statistics metrics
    metrics::describe_counter!(
        "pesto_sflow_datagrams_total",
        "Total number of sFlow datagrams received"
    );
    metrics::describe_counter!(
        "pesto_sflow_samples_total",
        "Total number of sFlow samples processed"
    );
    metrics::describe_counter!(
        "pesto_sflow_samples_received_total",
        "Total number of sFlow samples received by type (flow or counter)"
    );
    metrics::describe_counter!(
        "pesto_sflow_records_total",
        "Total number of sFlow records transmitted"
    );
}

pub async fn resolve_address(address: String) -> Result<SocketAddr> {
    match lookup_host(&address).await?.next() {
        Some(addr) => Ok(addr),
        None => anyhow::bail!("Failed to resolve address: {}", address),
    }
}

pub async fn configure() -> Result<AppConfig> {
    let cli = Cli::parse();

    // Set up tracing
    set_logging(&cli).map_err(|e| anyhow::anyhow!("Failed to set up logging: {}", e))?;

    // Resolve addresses
    let (sflow_addr, metrics_addr) = tokio::try_join!(
        resolve_address(cli.sflow_address),
        resolve_address(cli.metrics_address)
    )
    .map_err(|e| anyhow::anyhow!("Failed during initial address resolution: {}", e))?;

    let resolved_kafka_brokers = if !cli.kafka_disable {
        let mut brokers = Vec::new();
        for broker in cli.kafka_brokers {
            brokers.push(resolve_address(broker).await?);
        }
        brokers
    } else {
        Vec::new()
    };

    // Set up metrics
    set_metrics(metrics_addr);

    Ok(AppConfig {
        sflow: SFlowConfig { host: sflow_addr },
        kafka: KafkaConfig {
            disable: cli.kafka_disable,
            brokers: resolved_kafka_brokers,
            topic: cli.kafka_topic,
            auth_protocol: cli.kafka_auth_protocol,
            auth_sasl_username: cli.kafka_auth_sasl_username,
            auth_sasl_password: cli.kafka_auth_sasl_password,
            auth_sasl_mechanism: cli.kafka_auth_sasl_mechanism,
            message_max_bytes: cli.kafka_message_max_bytes,
            message_timeout_ms: cli.kafka_message_timeout_ms,
            batch_wait_time: cli.kafka_batch_wait_time,
            batch_wait_interval: cli.kafka_batch_wait_interval,
            mpsc_buffer_size: cli.kafka_mpsc_buffer_size,
        },
    })
}
