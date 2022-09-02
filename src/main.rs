use anyhow::Context;
use clap::Parser;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::time::sleep;

#[derive(Debug, Parser)]
#[clap(name = "ipfw")]
struct Args {
    /// Listen on this address
    listen_addr: String,
    /// Redirect traffic to this address
    target_addr: String,
    /// Only receive packets from IPv6 addresses
    #[clap(long)]
    v6_only: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args {
        listen_addr,
        target_addr,
        v6_only,
    } = Args::parse();
    let listen_addr = listen_addr.parse::<SocketAddr>()?;
    let target_addr = target_addr.parse::<SocketAddr>()?;

    let check_addr_handle = tokio::spawn(async move { check_addr(target_addr).await });
    let accept_handle =
        tokio::spawn(async move { accept(listen_addr, target_addr, v6_only).await });

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

async fn accept(
    listen_addr: SocketAddr,
    target_addr: SocketAddr,
    v6_only: bool,
) -> anyhow::Result<()> {
    let listener = if v6_only {
        let socket = Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_only_v6(true)?;
        socket.bind(&listen_addr.into())?;
        socket.set_nonblocking(true)?;
        socket.listen(1024)?;
        TcpListener::from_std(socket.into())?
    } else {
        TcpListener::bind(listen_addr).await?
    };

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
