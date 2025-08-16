mod adapter;
mod handler;
mod http;
mod path;
mod types;

use crate::adapter::RequestAdapter;
use crate::handler::Handler;
use crate::types::DefaultHeaderSelector;
use ::hyper::body::Bytes;
use ::hyper::service::service_fn;
use clap::Parser;
use colored::{ColoredString, Colorize};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rcgen::generate_simple_self_signed;
use std::convert::Infallible;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use tokio_rustls::rustls::{
    ClientConfig, DigitallySignedStruct, Error, ServerConfig, SignatureScheme,
};
use tokio_rustls::{TlsAcceptor, rustls};

#[derive(Parser, Debug)]
#[command(
name="serve",
bin_name="serve",
version,
about = "HTTP server that serves the static content in the current directory.",
long_about = None
)]
struct Args {
    #[arg(long)]
    prefix: Option<PathBuf>,
    #[arg(long)]
    // #[arg(default_value = "8443")]
    port: Option<u16>,
    #[arg(long)]
    // #[arg(default_value = "https://localhost")]
    forwarded_origin: Option<String>,
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
    server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, args.port.unwrap_or(443u16)))
        .await
        .expect("Failed to bind to port 443");
    println!(
        "{}",
        format!(
            "https://localhost{}/{prefix}",
            match args.port {
                Some(port) if port != 443 => format!(":{}", port),
                _ => "".to_string(),
            }
        )
        .bright_red()
        .underline()
    );
    let forwarded_uri = args
        .forwarded_origin
        .as_ref()
        .map(|it| Arc::new(Uri::from_str(it).expect("invalid forwarded origin")));
    let client = if args.forwarded_origin.is_some() {
        #[derive(Debug)]
        struct AcceptAllVerifier {
            server_name: String,
        }
        impl ServerCertVerifier for AcceptAllVerifier {
            fn verify_server_cert(
                &self,
                _end_entity: &CertificateDer<'_>,
                _intermediates: &[CertificateDer<'_>],
                server_name: &ServerName<'_>,
                _ocsp_response: &[u8],
                _now: UnixTime,
            ) -> Result<ServerCertVerified, Error> {
                let server_name = server_name.to_str();
                if server_name == "127.0.0.1"
                    || server_name == "::1"
                    || server_name == "localhost"
                    || server_name == self.server_name
                {
                    return Ok(ServerCertVerified::assertion());
                }
                Err(Error::General(format!("{server_name} not forwarded")))
            }
            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &CertificateDer<'_>,
                _dss: &DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, Error> {
                Ok(HandshakeSignatureValid::assertion())
            }
            fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
                vec![
                    SignatureScheme::RSA_PKCS1_SHA256,
                    SignatureScheme::ECDSA_NISTP256_SHA256,
                    SignatureScheme::ED25519,
                ]
            }
        }
        let config = ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier {
                server_name: forwarded_uri
                    .as_ref()
                    .unwrap()
                    .host()
                    .unwrap_or("localhost")
                    .to_string(),
            }))
            .with_no_client_auth();
        let client: Client<_, BoxBody<Bytes, hyper::Error>> = Client::builder(TokioExecutor::new())
            .build(
                HttpsConnectorBuilder::new()
                    .with_tls_config(config)
                    .https_only()
                    .enable_all_versions()
                    .build(),
            );
        Some(Arc::new(client))
    } else {
        None
    };
    loop {
        if let Ok((tcp_stream, _remote_address)) = listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            let client = client.clone();
            let forwarded_uri = forwarded_uri.clone();
            spawn(async move {
                if let Ok(tls_stream) = tls_acceptor.accept(tcp_stream).await {
                    let io = TokioIo::new(tls_stream);
                    let client = client.clone();
                    let forwarded_uri = forwarded_uri.clone();
                    let _ = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn({
                                let client = client.clone();
                                let forwarded_uri = forwarded_uri.clone();
                                move |request: Request<Incoming>| {
                                    let client = client.clone();
                                    let forwarded_uri = forwarded_uri.clone();
                                    async move {
                                        let header_selector = DefaultHeaderSelector;
                                        let handler = Handler {
                                            prefix,
                                            header_selector,
                                        };
                                        let (parts, body) = request.into_parts();
                                        let request =
                                            Request::from_parts(parts.clone(), empty_body());
                                        let request_message = format!(
                                            "{} {}",
                                            method_string(request.method().as_str().as_bytes()),
                                            request
                                                .uri()
                                                .path_and_query()
                                                .map(|it| it.as_str())
                                                .unwrap_or("/")
                                        );
                                        let response =
                                            handler.handle(RequestAdapter { inner: request }).await;
                                        let (response, forwarded) = if response
                                            .status()
                                            .is_client_error()
                                        {
                                            let body = body.boxed();
                                            if let Some(client) = client.as_ref() {
                                                let mut request = Request::from_parts(parts, body);
                                                let uri = request.uri();
                                                let forward_uri = Uri::builder();
                                                let forwarded_uri = forwarded_uri.unwrap();
                                                let forward_uri = if let Some(scheme) =
                                                    forwarded_uri.scheme().or_else(|| uri.scheme())
                                                {
                                                    forward_uri.scheme(scheme.clone())
                                                } else {
                                                    forward_uri
                                                };
                                                let forward_uri = if let Some(authority) =
                                                    forwarded_uri
                                                        .authority()
                                                        .or_else(|| uri.authority())
                                                {
                                                    forward_uri.authority(authority.clone())
                                                } else {
                                                    forward_uri
                                                };
                                                let forward_uri = if let Some(path_and_query) =
                                                    uri.path_and_query()
                                                {
                                                    forward_uri
                                                        .path_and_query(path_and_query.clone())
                                                } else {
                                                    forward_uri
                                                };
                                                let forward_uri = forward_uri
                                                    .build()
                                                    .expect("could not build forwarded uri");
                                                *request.uri_mut() = forward_uri;
                                                let forward_response =
                                                    client.request(request).await;
                                                match forward_response {
                                                    Ok(forward_response) => {
                                                        let (parts, body) =
                                                            forward_response.into_parts();
                                                        (
                                                            Response::from_parts(
                                                                parts,
                                                                body.boxed(),
                                                            ),
                                                            true,
                                                        )
                                                    }
                                                    Err(err) => {
                                                        println!(
                                                            "{}\n{err:?}",
                                                            "error on forwarded response".red()
                                                        );
                                                        (response, false)
                                                    }
                                                }
                                            } else {
                                                let _ = body.collect().await;
                                                (response, false)
                                            }
                                        } else {
                                            (response, false)
                                        };
                                        println!(
                                            "{} {} {request_message}",
                                            if forwarded { ">>" } else { "  " },
                                            status_string(&response.status()),
                                        );
                                        Ok::<Response<BoxBody<Bytes, hyper::Error>>, Infallible>(
                                            response,
                                        )
                                    }
                                }
                            }),
                        )
                        .await;
                }
            });
        }
    }
}

fn empty_body() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|err: Infallible| match err {})
        .boxed()
}

fn method_string(method: &[u8]) -> ColoredString {
    match method {
        b"HEAD" => "HEAD".yellow(),
        b"GET" => "GET".dimmed(),
        b"OPTIONS" => "OPTIONS".cyan(),
        _ => method.escape_ascii().to_string().red(),
    }
}

fn status_string(status: &StatusCode) -> ColoredString {
    if status.is_success() {
        status.as_u16().to_string().green()
    } else if status.is_redirection() {
        if status == &StatusCode::NOT_MODIFIED {
            status.as_u16().to_string().purple()
        } else {
            status.as_u16().to_string().blue()
        }
    } else if status.is_client_error() || status.is_server_error() {
        status.as_u16().to_string().red()
    } else {
        status.as_u16().to_string().normal()
    }
}
