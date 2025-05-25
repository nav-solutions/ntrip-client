use thiserror::Error;

mod credentials;
use credentials::NTRIPCredentials;

// use std::io::{Read, Write};

// use crate::coordinate::Coordinate;
// use crate::tcp_handler::TcpHandler;
// use base64::Engine as _;
// use std::io::{Read, Write};
// use std::time::{Duration, Instant};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use rtcm_rs::next_msg_frame;

#[derive(Debug, Error)]
pub enum NTRIPClientError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed to connect to server")]
    Connection,

    #[error("failed to send data to server")]
    Send,

    #[error("invalid response from server")]
    BadResponse,
}

#[cfg(feature = "log")]
use log::{error, info};

/// [NTRIPClient] allows to connect to a remote NTRIP server (v1 and v2 both supported),
/// to receiver RTCM messages. [NTRIPClient] supports both V1 and V2 NTRIP.
#[derive(Clone)]
pub struct NTRIPClient {
    /// Host (url)
    host: String,

    /// Network port
    port: u16,

    /// Name of the mountpoint
    mountpoint: String,

    /// Optional [NTRIPCredentials]
    credentials: Option<NTRIPCredentials>,
}

impl NTRIPClient {
    const GET_ICY_RESPONSE: &str = "ICY 200 OK\r\n";
    const GET_HTTPOK_RESPONSE: &str = "HTTP/1.1 200 OK\r\n";

    /// Creates a new [NTRIPClient]
    /// ## Input
    /// - host: url
    /// - port: network port
    /// - mountpoint: remote NTRIP "mountpoint"
    pub fn new(host: &str, port: u16, mountpoint: &str) -> Self {
        Self {
            port,
            credentials: None,
            host: host.to_string(),
            mountpoint: mountpoint.to_string(),
        }
    }

    /// Update [NTRIPClient] with desired credentials
    pub fn with_credentials(&self, user: &str, password: &str) -> Self {
        let mut s = self.clone();
        s.credentials = Some(NTRIPCredentials::new(user, password));
        s
    }

    /// Define [NTRIPClient] credentials, with mutable access.
    pub fn set_credentials(&mut self, user: &str, password: &str) {
        self.credentials = Some(NTRIPCredentials::new(user, password));
    }

    /// Deploy this [NTRIPClient] using tokio framework.
    pub async fn run(&mut self) -> Result<(), NTRIPClientError> {
        let mut ptr = 0;

        let mut buffer = [0u8; 1024];

        let get_icy_response_len: usize = Self::GET_ICY_RESPONSE.len();
        let get_httpok_response_len = Self::GET_HTTPOK_RESPONSE.len();

        let pkg_version = env!("CARGO_PKG_VERSION");

        #[cfg(feature = "log")]
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .map_err(|e| {
                error!("connection failed with: {}", e);
                NTRIPClientError::Connection
            })?;

        #[cfg(not(feature = "log"))]
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .map_err(|_| NTRIPClientError::Connection)?;

        // initial $GET request
        let mut request = format!(
            "GET /{} HTTP/1.0\r\n
            Host: {}\r\nNtrip-version: Ntrip/2.0\r\n
            User-Agent: rtk-rs/ntrip-client v{}\r\n
            Connection: close\r\n
            Accept: */*\r\n",
            self.mountpoint, self.host, pkg_version,
        );

        if let Some(creds) = &self.credentials {
            request.push_str(&format!("Authorization: Basic{}\r\n", &creds.encode()));
        }

        #[cfg(feature = "log")]
        stream.write_all(request.as_bytes()).await.map_err(|e| {
            #[cfg(feature = "log")]
            error!("write error: {}", e);
            NTRIPClientError::Send
        })?;

        #[cfg(not(feature = "log"))]
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|_| NTRIPClientError::Send)?;

        // response verification
        loop {
            let size = stream.read(&mut buffer[ptr..]).await?;
            if size == 0 {
                break;
            }
            ptr += size;
        }

        if ptr < get_icy_response_len && ptr < get_httpok_response_len {
            #[cfg(feature = "log")]
            error!("invalid server response");
            return Err(NTRIPClientError::BadResponse);
        }

        let response = String::from_utf8_lossy(&buffer[..ptr]);

        if !response.starts_with(Self::GET_ICY_RESPONSE) {
            if !response.starts_with(Self::GET_HTTPOK_RESPONSE) {
                // #[cfg(feature = "log")]
                println!("invalid response from server: \"{}\"", response);
                return Err(NTRIPClientError::BadResponse);
            }
        }

        #[cfg(feature = "log")]
        info!(
            "rtk-rs/ntrip-client v{} - connected to {}",
            pkg_version, self.host
        );

        loop {
            ptr = 0;
            let size = stream.read(&mut buffer[ptr..]).await?;

            if size == 0 {
                #[cfg(feature = "log")]
                error!("{} - connectoion closed", self.host);
                return Ok(());
            }

            loop {
                let (consumed, msg) = next_msg_frame(&buffer[ptr..]);

                if consumed == 0 {
                    break;
                }

                ptr += consumed;

                if let Some(msg) = msg {
                    println!("Found {:?}", msg.get_message());
                }
            }
        }
    }
}

// #[cfg(test)]
// mod test {
//     use crate::NTRIPClient;
// 
//     #[tokio::test]
//     async fn test_simple_connection() {
//         let mut client = NTRIPClient::new("caster.centipede.fr", 2101, "ENSMM")
//             .with_credentials("centipede", "centipede");
// 
//         client.run().await.unwrap_or_else(|e| {
//             panic!("run() failed with {}", e);
//         });
//     }
// }
