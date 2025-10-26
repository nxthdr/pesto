use anyhow::Result;
use clap::Parser;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(author, version, about = "sFlow producer for integration testing", long_about = None)]
struct Cli {
    /// Path to the .bin file containing sFlow datagram(s)
    #[arg(short, long)]
    file: PathBuf,

    /// Target sFlow collector address
    #[arg(short, long, default_value = "127.0.0.1:6343")]
    target: SocketAddr,

    /// Number of times to send the datagram(s)
    #[arg(short, long, default_value_t = 1)]
    count: usize,

    /// Interval between sends in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Read the binary file
    let data = fs::read(&cli.file)?;
    println!(
        "Loaded {} bytes from {}",
        data.len(),
        cli.file.display()
    );

    // Create UDP socket
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    println!("Sending to {}", cli.target);

    // Send the datagram(s) multiple times
    for i in 1..=cli.count {
        socket.send_to(&data, cli.target).await?;
        println!("Sent datagram {} ({} bytes)", i, data.len());

        if i < cli.count {
            sleep(Duration::from_millis(cli.interval)).await;
        }
    }

    println!("Done!");
    Ok(())
}
