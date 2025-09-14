use reqwest::header::{InvalidHeaderValue, ToStrError};
use rustls::pki_types::InvalidDnsNameError;

/// NTRIP client error types
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

    #[error("Invalid URL")]
    InvalidUrl,

    #[error("Invalid port number")]
    InvalidPort,
}
