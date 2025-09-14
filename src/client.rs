//! NTRIP Client implementation

use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use futures::Stream;
use http::{
    header::{InvalidHeaderValue, ToStrError, USER_AGENT},
    HeaderMap, HeaderValue, Method,
};
use rtcm_rs::{Message, MessageFrame};
use rustls::pki_types::{InvalidDnsNameError, ServerName};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    select,
    sync::{
        broadcast::Sender as BroadcastSender,
        mpsc::{unbounded_channel, UnboundedReceiver},
    },
    task::JoinHandle,
};
use tokio_rustls::TlsConnector;
use tracing::{debug, error, trace, warn};

use crate::{
    config::{NtripConfig, NtripCredentials},
    snip::ServerInfo,
};

/// NTRIP Client, used to connect to an NTRIP (RTCM) service
pub struct NtripClient {
    config: NtripConfig,
    creds: NtripCredentials,
}

/// NTRIP Mount handle, used to stream RTCM messages from an NTRIP service
pub struct NtripHandle {
    _rx_handle: tokio::task::JoinHandle<()>,
    ntrip_rx: UnboundedReceiver<Message>,
}

#[derive(Debug, thiserror::Error)]
pub enum NtripClientError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Invalid header value {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Invalid DNS name {0}")]
    InvalidDnsName(#[from] InvalidDnsNameError),

    #[error("Header ToStrError error {0}")]
    ToStrError(#[from] ToStrError),

    #[error("Response error")]
    ResponseError(String),
}

impl NtripClient {
    pub async fn new(
        config: NtripConfig,
        creds: NtripCredentials,
    ) -> Result<Self, NtripClientError> {
        Ok(NtripClient { config, creds })
    }

    /// List available mounts on the NTRIP server
    pub async fn list_mounts(&mut self) -> Result<ServerInfo, NtripClientError> {
        let client = reqwest::Client::builder()
            .http1_ignore_invalid_headers_in_responses(true)
            .http09_responses()
            .user_agent(format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;

        // TODO: auth etc.
        let proto = if self.config.use_tls { "https" } else { "http" };

        let req = client
            .request(
                Method::GET,
                format!("{}://{}:{}", proto, self.config.host, self.config.port),
            )
            .header("Ntrip-Version", "NTRIP/2.0")
            .build()?;

        let res = client.execute(req).await?;

        debug!("Fetched NTRIP response: {:?}", res.status());

        let body = res.text().await?;

        let lines = body.lines().collect::<Vec<&str>>();

        let snip_info = ServerInfo::parse(lines.iter().cloned());

        Ok(snip_info)
    }

    pub async fn mount(
        &mut self,
        mount: impl ToString,
        exit_tx: BroadcastSender<()>,
    ) -> Result<NtripHandle, NtripClientError> {
        debug!(
            "Connecting to NTRIP server {}/{}",
            self.config.url(),
            mount.to_string()
        );

        let sock = TcpStream::connect(&self.config.url()).await?;

        let (rx_handle, ntrip_rx) = match self.config.use_tls {
            true => {
                debug!("Using TLS connection");

                let mut root_cert_store = rustls::RootCertStore::empty();
                root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

                let tls_config = rustls::ClientConfig::builder()
                    .with_root_certificates(root_cert_store)
                    .with_no_client_auth();
                let connector = TlsConnector::from(Arc::new(tls_config));
                let dnsname = ServerName::try_from(self.config.host.clone())?;

                let tls_sock = connector.connect(dnsname, sock).await?;

                Self::handle_connection(
                    &self.config,
                    &self.creds,
                    &mount.to_string(),
                    exit_tx.clone(),
                    tls_sock,
                )
                .await?
            },
            false => {
                debug!("Using plain TCP connection");

                Self::handle_connection(
                    &self.config,
                    &self.creds,
                    &mount.to_string(),
                    exit_tx.clone(),
                    sock,
                )
                .await?
            },
        };

        Ok(NtripHandle {
            _rx_handle: rx_handle,
            ntrip_rx,
        })
    }

    pub async fn handle_connection(
        config: &NtripConfig,
        creds: &NtripCredentials,
        mount: &str,
        exit_tx: BroadcastSender<()>,
        mut sock: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    ) -> Result<(JoinHandle<()>, UnboundedReceiver<Message>), NtripClientError> {
        // Setup HTTP headers
        let mut headers = HeaderMap::new();
        headers.append(
            USER_AGENT,
            HeaderValue::from_str(&format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))?,
        );

        headers.append("Ntrip-Version", HeaderValue::from_static("NTRIP/2.0"));
        headers.append("Accept", HeaderValue::from_static("*/*"));
        headers.append("Connection", HeaderValue::from_static("close"));

        // If we have credentials, add the Authorization header
        if !creds.user.is_empty() {
            let auth = general_purpose::STANDARD.encode(format!("{}:{}", creds.user, creds.pass));
            headers.append(
                "Authorization",
                HeaderValue::from_str(&format!("Basic {}", auth))?,
            );
        }

        debug!("Headers: {:#?}", headers);

        // Write HTTP request
        debug!("Write HTTP request");
        sock.write_all(format!("GET /{} HTTP/1.0\r\n", mount).as_bytes())
            .await?;
        sock.write_all(format!("Host: {}\r\n", config.url()).as_bytes())
            .await?;

        // Write HTTP headers
        debug!("Writing headers");
        for h in headers.iter() {
            sock.write_all(format!("{}: {}\r\n", h.0.as_str(), h.1.to_str()?).as_bytes())
                .await?;
        }

        sock.write_all(b"\r\n").await?;
        sock.flush().await?;

        debug!("Reading response");
        let mut buff = Vec::with_capacity(1024);

        // Perform a first read to get the response status
        let n = sock.read_buf(&mut buff).await?;
        debug!("Read {} bytes, current buffer {} bytes", n, buff.len());

        // Parse out response status
        let r = String::from_utf8_lossy(&buff[..n]);
        match r.lines().next() {
            Some(status) if status.contains("200 OK") => {
                debug!("Got 200 OK response");
            },
            Some(status) => {
                error!("NTRIP server returned error: {}", status);
                return Err(NtripClientError::ResponseError(status.to_string()));
            },
            None => {
                error!("NTRIP server returned empty response");
                return Err(NtripClientError::ResponseError("empty response".into()));
            },
        }

        // Flush buffer until the first RTCM message (0xd3)
        if let Some(i) = buff.iter().enumerate().find(|(_i, b)| **b == 0xd3) {
            debug!(
                "Trimming buffer to next potential frame start at index {}",
                i.0
            );
            let _ = buff.drain(..i.0);
        }

        // Spawn a task to handle incoming NTRIP data

        let (ntrip_tx, ntrip_rx) = unbounded_channel();
        let mut exit_rx = exit_tx.subscribe();
        let rx_handle = tokio::task::spawn(async move {
            // Track parse errors so we can drop data (or abort) if needed
            let mut error_count = 0;

            'listener: loop {
                select! {
                    n = sock.read_buf(&mut buff) => match n {
                        Ok(n) => {
                            debug!("Read {} bytes, current buffer {} bytes", n, buff.len());
                            trace!("Appended {:02x?}", &buff[buff.len()-n..][..n]);

                            // Handle zero length read (connection closed)
                            if n == 0 {
                                warn!("Zero length response");
                                break 'listener;
                            }

                            // Trim any non-message data from the start of the buffer
                            if buff[0] != 0xd3 {
                                if let Some(i) = buff.iter().enumerate().find(|(_i, b)| **b == 0xd3) {
                                    warn!("Trimming buffer to next potential frame start at index {}", i.0);
                                    buff.drain(..i.0);

                                    assert_eq!(buff[0], 0xd3);
                                }
                            }

                            // While we have enough data for a header,
                            // parse out RTCM messages
                            while buff.len() > 6 {
                                // Attempt to parse frames
                                match MessageFrame::new(&buff[..]) {
                                    Ok(f) => {
                                        // Parse out message from frame
                                        let m = f.get_message();

                                        debug!("Parsed RTCM message: {:?} (consumed {} bytes)", m, f.frame_len());

                                        // Emit message
                                        ntrip_tx.send(m).unwrap();

                                        // Remove parsed data from the buffer
                                        let _ = buff.drain(..f.frame_len());

                                        // Reset error counter
                                        error_count = 0;
                                    },
                                    Err(e) => {
                                        warn!("RTCM parse error: {} (count: {})", e, error_count);

                                        // Update error counter
                                        error_count += 1;

                                        // If we keep getting errors, abort the connection
                                        if error_count >= 5 {
                                            error!("Too many parse errors, closing connection");
                                            break 'listener;
                                        }

                                        break;
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            error!("socket read error: {}", e);
                            break;
                        },
                    },
                    _ = exit_rx.recv() => {
                        error!("Exiting NTRIP read loop on signal");
                        break;
                    }
                }
            }

            warn!("NTRIP read loop exiting");

            if !buff.is_empty() {
                warn!("Dropping {} bytes of unparsed data", buff.len());

                if let Ok(s) = String::from_utf8(buff) {
                    debug!("Unparsed data:\r\n{}", s);
                }
            }
        });

        Ok((rx_handle, ntrip_rx))
    }
}

/// [Stream] NTRIP [Message]'s from an [NtripHandle]
impl Stream for NtripHandle {
    type Item = Message;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.ntrip_rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use futures::StreamExt;
    use tracing::debug;

    use super::*;
    use crate::config::NtripCredentials;

    fn setup_logging() {
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(tracing::level_filters::LevelFilter::DEBUG)
            .try_init();
    }

    #[tokio::test]
    #[ignore = "Requires NTRIP config from the environment"]
    async fn test_ntrip_client() {
        setup_logging();

        debug!("Connecting to NTRIP server");

        let (exit_tx, _exit_rx) = tokio::sync::broadcast::channel(1);

        let mount = env::var("NTRIP_MOUNT").unwrap_or("ARGOACU".to_string());
        let config = env::var("NTRIP_HOST")
            .unwrap_or("rtk2go".to_string())
            .parse::<NtripConfig>()
            .unwrap();
        let creds = NtripCredentials {
            user: env::var("NTRIP_USER").unwrap_or("user".into()),
            pass: env::var("NTRIP_PASS").unwrap_or("pass".into()),
        };

        let mut client = NtripClient::new(config, creds).await.unwrap();

        let mut h = client
            .mount(mount.to_string(), exit_tx.clone())
            .await
            .unwrap();

        for _i in 0..10 {
            let m = h.next().await.unwrap();
            debug!("Got RTCM message: {:?}", m);
        }

        let _ = exit_tx.send(());
    }
}
