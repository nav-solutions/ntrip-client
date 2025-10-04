//! NTRIP Client library
//!
//! Provides an async NTRIP client for listing mounts and connecting to RTCM services

pub mod config;

pub mod snip;

mod error;
pub use error::NtripClientError;

mod client;
pub use client::NtripClient;
