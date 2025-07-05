mod adapter;
mod handler;
mod http;
mod path;
mod types;

use crate::adapter::RequestAdapter;
use crate::handler::Handler;
use crate::types::DefaultHeaderSelector;
use ::hyper::body::Bytes;
use ::hyper::server::conn::http2;
use ::hyper::service::service_fn;
use clap::Parser;
use colored::Colorize;
use http_body_util::combinators::BoxBody;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rcgen::generate_simple_self_signed;
use std::convert::Infallible;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::PrivateKeyDer;

#[derive(Parser, Debug)]
#[command(
name="serve",
bin_name="serve",
version,
about = "HTTP server that serves the static content in the current directory.",
long_about = None
)]
struct Args {
    #[arg(short, long)]
    prefix: Option<PathBuf>,
}
#[tokio::main]
async fn main() {
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).ok();
    let args = Args::parse();
    let prefix = args
        .prefix
        .map(|it| it.to_str().expect("invalid prefix").to_string())
        .unwrap_or("/".to_string());
    let prefix = prefix.strip_prefix('/').unwrap_or(&prefix).to_string();
    let prefix = prefix.strip_suffix('/').unwrap_or(&prefix).to_string();
    let prefix: &'static str = prefix.leak();
    let domains: Vec<String> = vec![
        format!("{}", Ipv4Addr::LOCALHOST),
        format!("{}", Ipv6Addr::LOCALHOST),
    ];
    let cert = Box::leak(Box::new(
        generate_simple_self_signed(domains)
            .expect("failed to generate self-signed certificate for localhost"),
    ));
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivateKeyDer::Pkcs8(cert.signing_key.serialized_der().into()),
        )
        .expect("Failed to create certificate.");
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 443u16))
        .await
        .expect("Failed to bind to port 443");
    println!(
        "{}",
        format!("https://localhost/{prefix}")
            .bright_red()
            .underline()
    );
    loop {
        if let Ok((tcp_stream, _remote_address)) = listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            spawn(async move {
                if let Ok(tls_stream) = tls_acceptor.accept(tcp_stream).await {
                    let io = TokioIo::new(tls_stream);
                    let _ = http2::Builder::new(TokioExecutor::new())
                        .serve_connection(
                            io,
                            service_fn(move |request| async move {
                                let header_selector = DefaultHeaderSelector;
                                let handler = Handler {
                                    prefix,
                                    header_selector,
                                };
                                Ok::<hyper::Response<BoxBody<Bytes, hyper::Error>>, Infallible>(
                                    handler.handle(RequestAdapter { inner: request }).await,
                                )
                            }),
                        )
                        .await;
                }
            });
        }
    }
}
