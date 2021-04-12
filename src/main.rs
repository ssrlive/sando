mod request_handler;
mod tunnel;

use argh::FromArgs;
use regex::Regex;
use request_handler::ServerResponse;
use std::fs::File;
use std::io::prelude::*;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use tokio::io::{self, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_native_tls::TlsAcceptor;
use tokio_native_tls::{native_tls, TlsStream};
use tunnel::{Tunnel, TunnelStats};

#[derive(FromArgs)]
#[argh(description = "HTTPS server settings")]
struct Options {
    #[argh(positional)]
    addr: String,

    #[argh(option, short = 'c')]
    #[argh(description = "the certificate file in pkcs12 format for the server")]
    pkcs12: PathBuf,

    #[argh(option, short = 'p')]
    #[argh(description = "the password for the pkcs12 file")]
    password: String,

    #[argh(option, short = 'd', default = "String::from(\".*\")")]
    #[argh(description = "the domain pattern, in regex expression, of the proxy destination")]
    destination_pattern: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let options: Options = argh::from_env();
    let addr = options
        .addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let destination_pattern = Regex::new(&options.destination_pattern)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let identity = load_identity(&options.pkcs12, &options.password)?;

    let tls_acceptor = tokio_native_tls::TlsAcceptor::from(
        native_tls::TlsAcceptor::builder(identity)
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    );

    let tcp_listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, peer_addr) = tcp_listener.accept().await?;
        println!("Connection from: {}", peer_addr);
        let acceptor = tls_acceptor.clone();
        let pattern = destination_pattern.clone();
        tokio::spawn(async move {
            match handle_connection(acceptor, stream, pattern).await {
                Err(e) => {
                    println!("Error: {:?}", e);
                }
                Ok(ConnectionResult::InvalidRequest(method)) => {
                    println!("{} is not support", method);
                }
                Ok(ConnectionResult::Connect(ConnectResult::InvalidDestination(dest))) => {
                    println!("{} is not a allowed destination", dest);
                }
                Ok(ConnectionResult::Connect(ConnectResult::Success(client, dest, stats))) => {
                    assert_eq!(peer_addr, client);
                    println!(
                        "{} <-> Proxy <-> {}:\n\t{} -> {}: {} bytes\n\t{} -> {}: {} bytes",
                        client,
                        dest,
                        client,
                        dest,
                        stats.client_to_dest,
                        dest,
                        client,
                        stats.dest_to_client
                    );
                }
            }
            println!("Connection from {} is ended", peer_addr);
        });
    }
}

// The result of the HTTP request
enum ConnectionResult {
    InvalidRequest(String),
    Connect(ConnectResult),
}

// The result of the HTTP CONNECT request
enum ConnectResult {
    InvalidDestination(String),
    Success(SocketAddr, SocketAddr, TunnelStats),
}

async fn handle_connection(
    tls_acceptor: TlsAcceptor,
    tcp_stream: TcpStream,
    destination_pattern: Regex,
) -> io::Result<ConnectionResult> {
    let client_addr = tcp_stream.peer_addr()?;
    let handshake = tls_acceptor.accept(tcp_stream);
    let mut tls_stream = handshake
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e))?;
    let req = request_handler::get_request(&mut tls_stream).await?;
    match req.method.name.as_str() {
        "CONNECT" => {
            let r = handle_connect_request(
                tls_stream,
                client_addr,
                req.method.uri,
                destination_pattern,
            )
            .await?;
            Ok(ConnectionResult::Connect(r))
        }
        method => {
            end_invalid_request(tls_stream, ServerResponse::MethodNotAllowed).await?;
            Ok(ConnectionResult::InvalidRequest(method.to_string()))
        }
    }
}

async fn handle_connect_request(
    client: TlsStream<TcpStream>,
    client_addr: SocketAddr,
    destination_uri: String,
    destination_pattern: Regex,
) -> io::Result<ConnectResult> {
    match destination_uri.to_socket_addrs()?.next() {
        None => {
            end_invalid_request(client, ServerResponse::BadRequest).await?;
            Ok(ConnectResult::InvalidDestination(destination_uri))
        }
        Some(dest_addr) => {
            if !destination_pattern.is_match(&destination_uri) {
                end_invalid_request(client, ServerResponse::Forbidden).await?;
                return Ok(ConnectResult::InvalidDestination(destination_uri));
            }
            let stats = process_connect_request(client, client_addr, dest_addr).await?;
            Ok(ConnectResult::Success(client_addr, dest_addr, stats))
        }
    }
}

async fn process_connect_request(
    mut client: TlsStream<TcpStream>,
    client_addr: SocketAddr,
    dest_addr: SocketAddr,
) -> io::Result<TunnelStats> {
    let client_name = format!("{}", client_addr);
    let dest_name = format!("{}", dest_addr);
    let dest = TcpStream::connect(dest_addr).await?;
    request_handler::send_response(&mut client, ServerResponse::Ok).await?;
    let mut tunnel = Tunnel::new(client_name, client, dest_name, dest);
    Ok(tunnel.start().await?)
}

async fn end_invalid_request(
    mut client: TlsStream<TcpStream>,
    res: ServerResponse,
) -> io::Result<()> {
    request_handler::send_response(&mut client, res).await?;
    client.shutdown().await?;
    Ok(())
}

fn load_identity(path: &Path, password: &str) -> io::Result<native_tls::Identity> {
    let mut file = File::open(path)?;
    let mut identity = vec![];
    file.read_to_end(&mut identity)?;
    native_tls::Identity::from_pkcs12(&identity, password)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}
