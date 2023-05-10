use clap::Parser;
use core::task::{Context, Poll};
use futures_util::ready;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::vec::Vec;
use std::{fs, io};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::{self};

#[derive(Parser)]
#[clap(name = "Fake Explorers HTTPS server")]
#[clap(author = "DFINITY Execution Team")]
struct Cli {
    #[clap(long, default_value = "127.0.0.1:8080")]
    addr: SocketAddr,

    #[clap(long, default_value = "./src/certificate.pem")]
    cert: PathBuf,

    #[clap(long, default_value = "./src/private-key.rsa")]
    key: PathBuf,
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), response_text(req.uri().path())) {
        (&Method::GET, Some(text)) => Ok(Response::new(Body::from(text))),
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

fn main() {
    // Serve an echo service over HTTPS, with proper error handling.
    if let Err(e) = run_server() {
        eprintln!("FAILED: {}", e);
        std::process::exit(1);
    }
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    // Build TLS configuration.
    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs(&cli.cert)?;
        // Load private key.
        let key = load_private_key(&cli.key)?;
        // Do not use client certificate authentication.
        let mut cfg = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| error(format!("{}", e)))?;
        // Configure ALPN to accept HTTP/2, HTTP/1.1, and HTTP/1.0 in that order.
        cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
        Arc::new(cfg)
    };

    // Create a TCP listener via tokio.
    let incoming = AddrIncoming::bind(&cli.addr)?;
    let service =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });
    let server = Server::builder(TlsAcceptor::new(tls_cfg, incoming)).serve(service);

    // Run the future, keep going until an error occurs.
    println!("Starting to serve on https://{}.", cli.addr);
    server.await?;

    Ok(())
}

fn response_text(path: &str) -> Option<String> {
    let height = 700_100;
    let response = match path {
        "/status" => String::from("OK"),
        "/api.bitaps.com/btc/v1/blockchain/block/last"
        | "/api.bitaps.com/btc/testnet/v1/blockchain/block/last" => api_bitaps_com_response(height),
        "/api.blockchair.com/bitcoin/stats" | "/api.blockchair.com/bitcoin/testnet/stats" => {
            api_blockchair_com_response(height)
        }
        "/api.blockcypher.com/v1/btc/main" | "/api.blockcypher.com/v1/btc/test3" => {
            api_blockcypher_com_response(height)
        }
        "/ghsi2-tqaaa-aaaan-aaaca-cai.raw.ic0.app/metrics"
        | "/g4xu7-jiaaa-aaaan-aaaaq-cai.raw.ic0.app/metrics" => bitcoin_canister_response(height),
        "/blockchain.info/q/latesthash" => blockchain_info_hash_response(),
        "/blockchain.info/q/getblockcount" => blockchain_info_height_response(height),
        "/blockstream.info/api/blocks/tip/hash"
        | "/blockstream.info/testnet/api/blocks/tip/hash" => blockstream_info_hash_response(),
        "/blockstream.info/api/blocks/tip/height"
        | "/blockstream.info/testnet/api/blocks/tip/height" => {
            blockstream_info_height_response(height)
        }
        "/chain.api.btc.com/v3/block/latest" => chain_api_btc_com_response(height),
        _ => return None,
    };

    Some(response)
}

fn api_bitaps_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "height": height
        },
    }))
    .unwrap()
}

fn api_blockchair_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "best_block_height": height
        },
    }))
    .unwrap()
}

fn api_blockcypher_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({ "height": height })).unwrap()
}

fn bitcoin_canister_response(height: u64) -> String {
    format!(r#"main_chain_height {height} 1680014894644"#)
}

fn blockchain_info_hash_response() -> String {
    r#"0000000000000000000aaa444444444444444444444444444444444444444444"#.to_string()
}

fn blockchain_info_height_response(height: u64) -> String {
    format!(r#"{height}"#)
}

fn blockstream_info_hash_response() -> String {
    r#"0000000000000000000aaa444444444444444444444444444444444444444444"#.to_string()
}

fn blockstream_info_height_response(height: u64) -> String {
    format!(r#"{height}"#)
}

fn chain_api_btc_com_response(height: u64) -> String {
    serde_json::to_string_pretty(&json!({
        "data": {
            "height": height
        },
    }))
    .unwrap()
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

// Load public certificate from file.
fn load_certs(filename: &PathBuf) -> io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {:?}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|_| error("failed to load certificate".into()))?;
    Ok(certs.into_iter().map(rustls::Certificate).collect())
}

// Load private key from file.
fn load_private_key(filename: &PathBuf) -> io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {:?}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = rustls_pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }

    Ok(rustls::PrivateKey(keys[0].clone()))
}

enum State {
    Handshaking(tokio_rustls::Accept<AddrStream>),
    Streaming(tokio_rustls::server::TlsStream<AddrStream>),
}

// tokio_rustls::server::TlsStream doesn't expose constructor methods,
// so we have to TlsAcceptor::accept and handshake to have access to it
// TlsStream implements AsyncRead/AsyncWrite handshaking tokio_rustls::Accept first
pub struct TlsStream {
    state: State,
}

impl TlsStream {
    fn new(stream: AddrStream, config: Arc<ServerConfig>) -> TlsStream {
        let accept = tokio_rustls::TlsAcceptor::from(config).accept(stream);
        TlsStream {
            state: State::Handshaking(accept),
        }
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_read(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_write(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct TlsAcceptor {
    config: Arc<ServerConfig>,
    incoming: AddrIncoming,
}

impl TlsAcceptor {
    pub fn new(config: Arc<ServerConfig>, incoming: AddrIncoming) -> TlsAcceptor {
        TlsAcceptor { config, incoming }
    }
}

impl Accept for TlsAcceptor {
    type Conn = TlsStream;
    type Error = io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let pin = self.get_mut();
        match ready!(Pin::new(&mut pin.incoming).poll_accept(cx)) {
            Some(Ok(sock)) => Poll::Ready(Some(Ok(TlsStream::new(sock, pin.config.clone())))),
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}
