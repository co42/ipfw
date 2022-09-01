use anyhow::Context;
use clap::Parser;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::time::sleep;

#[derive(Debug, Parser)]
#[clap(name = "ipfw")]
struct Args {
    source_addr: String,
    target_addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source_addr = args.source_addr.parse::<SocketAddr>()?;
    let target_addr = args.target_addr.parse::<SocketAddr>()?;

    let check_addr_handle = tokio::spawn(async move { check_addr(target_addr).await });
    let accept_handle = tokio::spawn(async move { accept(source_addr, target_addr).await });

    select! {
        result = check_addr_handle => {
            result??;
        }
        result = accept_handle => {
            result??;
        }
    }

    Ok(())
}

async fn check_addr(addr: SocketAddr) -> anyhow::Result<()> {
    let mut buf = [0u8];
    loop {
        // Try to connect
        let socket = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("Connecting to {addr:?}"))?;

        // Check connection
        while socket.peek(&mut buf).await.map(|n| n != 0).unwrap_or(true) {
            sleep(Duration::from_millis(100)).await;
        }
    }
}

async fn accept(source_addr: SocketAddr, target_addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(source_addr).await?;
    println!("Listening on {source_addr}");

    loop {
        let mut peer_stream = match listener.accept().await {
            Ok((peer_stream, peer_addr)) => {
                println!("Accept {peer_addr}");
                peer_stream
            }
            Err(err) => {
                println!("Accept error: {err:?}");
                continue;
            }
        };
        let mut dest_stream = TcpStream::connect(&target_addr)
            .await
            .with_context(|| format!("Connecting to {target_addr:?}"))?;
        tokio::spawn(async move {
            if let Err(err) = copy_bidirectional(&mut peer_stream, &mut dest_stream).await {
                println!("Forward error: {err:?}");
            }
        });
    }
}
