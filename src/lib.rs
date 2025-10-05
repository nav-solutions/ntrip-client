//! NTRIP Client library
//!
//! Provides an async NTRIP client for listing mounts and connecting to RTCM services

pub mod config;
pub use config::*;

pub mod snip;
pub use snip::*;

mod error;
pub use error::NtripClientError;

mod client;
pub use client::NtripClient;
