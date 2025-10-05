//! NTRIP client configuration objects

use std::str::FromStr;

use strum::{Display, EnumString, VariantNames};

use crate::NtripClientError;

/// NTRIP (Networked Transport of RTCM via Internet Protocol) configuration
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NtripConfig {
    /// Host name or IP address of the NTRIP server
    #[cfg_attr(
        feature = "clap",
        clap(long = "ntrip-host", env = "NTRIP_HOST", default_value = "rtk2go.com")
    )]
    pub host: String,

    /// Port number of the NTRIP server
    #[cfg_attr(
        feature = "clap",
        clap(long = "ntrip-port", env = "NTRIP_PORT", default_value_t = 2101)
    )]
    pub port: u16,

    /// Use TLS / SSL for the NTRIP connection
    #[cfg_attr(
        feature = "clap",
        clap(long = "ntrip-use-tls", env = "NTRIP_USE_TLS", default_value_t = false)
    )]
    pub use_tls: bool,
}

impl Default for NtripConfig {
    /// Builds a default [NtripConfig] ready to connect to [RtcmProvider::Centipede] network.
    /// The network does not requires SSL.
    fn default() -> Self {
        Self::from_provider(RtcmProvider::Centipede)
    }
}

impl NtripConfig {
    /// Generate a connection URL ("host:port") from the NtripConfig
    pub fn to_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Prepares an [NtripConfig] for one of our predefined [RtcmProvider]s
    pub fn from_provider(network: RtcmProvider) -> Self {
        Self {
            host: network.host().to_string(),
            port: network.port(),
            use_tls: network.uses_tls(),
        }
    }

    /// Copies and returns [NtripConfig] with updated "host" IP address
    pub fn with_host(&self, address: &str) -> Self {
        let mut s = self.clone();
        s.host = address.to_string();
        s
    }

    /// Copies and returns [NtripConfig] with updated port number
    pub fn with_port(&self, port: u16) -> Self {
        let mut s = self.clone();
        s.port = port;
        s
    }

    /// Copies and returns [NtripConfig] with TLS/SSL active
    pub fn with_tls(&self) -> Self {
        let mut s = self.clone();
        s.use_tls = true;
        s
    }

    /// Copies and returns [NtripConfig] without TLS/SSL active
    pub fn without_tls(&self) -> Self {
        let mut s = self.clone();
        s.use_tls = false;
        s
    }
}

/// Credentials for an NTRIP (RTCM) service
#[derive(Clone, Default, PartialEq, Debug)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct NtripCredentials {
    /// Username for the NTRIP service
    #[cfg_attr(feature = "clap", clap(long = "ntrip-user", env = "NTRIP_USER"))]
    pub user: String,

    /// Password for the NTRIP service
    #[cfg_attr(
        feature = "clap",
        clap(long = "ntrip-pass", env = "NTRIP_PASS", default_value = "")
    )]
    pub pass: String,
}

impl NtripCredentials {
    /// Copies and returns updated [NtripCredentials]
    pub fn with_username(&self, username: &str) -> Self {
        let mut s = self.clone();
        s.user = username.to_string();
        s
    }

    /// Copies and returns updated [NtripCredentials]
    pub fn with_password(&self, password: &str) -> Self {
        let mut s = self.clone();
        s.pass = password.to_string();
        s
    }
}

/// Common RTCM data providers
#[derive(Clone, PartialEq, Debug, EnumString, Display, VariantNames)]
pub enum RtcmProvider {
    /// Land Information New Zealand
    ///
    /// Note: requires credentials
    #[strum(serialize = "linz")]
    Linz,
    /// RTK2GO.com free service
    #[strum(serialize = "rtk2go")]
    Rtk2Go,
    /// Positioning Australia
    ///
    /// Note: requires credentials and TLS
    #[strum(serialize = "posau")]
    PosAu,
    /// Centipede FR
    #[strum(serialize = "centipede")]
    Centipede,
}

impl RtcmProvider {
    /// Fetch the hostname for the provider
    pub fn host(&self) -> &str {
        match self {
            RtcmProvider::Linz => "positionz-rt.linz.govt.nz",
            RtcmProvider::Rtk2Go => "rtk2go.com",
            RtcmProvider::PosAu => "ntrip.data.gnss.ga.gov.au",
            RtcmProvider::Centipede => "caster.centipede.fr",
        }
    }

    /// Fetch the TCP port for the provider
    pub fn port(&self) -> u16 {
        match self {
            RtcmProvider::Linz => 2101,
            RtcmProvider::Rtk2Go => 2101,
            RtcmProvider::PosAu => 443,
            RtcmProvider::Centipede => 2101,
        }
    }

    /// Returns true if this [RtcmProvider] requires TLS/SSL.
    pub fn uses_tls(&self) -> bool {
        match self {
            RtcmProvider::Linz => false,
            RtcmProvider::Rtk2Go => false,
            RtcmProvider::PosAu => true,
            RtcmProvider::Centipede => false,
        }
    }
}

/// Parse an [NtripConfig] from a URL string
///
/// For example:
/// ```
/// # use ntrip_client::config::NtripConfig;
///
/// let cfg = "ntrip://rtk2go.com:2101".parse::<NtripConfig>().unwrap();
///
/// assert_eq!(cfg.host, "rtk2go.com");
/// assert_eq!(cfg.port, 2101);
/// assert_eq!(cfg.use_tls, false);
/// ```
///
/// This also matches on [RtcmProvider]'s for convenience.
/// ```
/// # use ntrip_client::config::NtripConfig;
///
/// let cfg = "linz".parse::<NtripConfig>().unwrap();
///
/// assert_eq!(cfg.host, "positionz-rt.linz.govt.nz");
/// assert_eq!(cfg.port, 2101);
/// assert_eq!(cfg.use_tls, false);
/// ```
impl FromStr for NtripConfig {
    type Err = NtripClientError;

    /// Parse an [NtripConfig] from a URL string
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Match on known providers
        if let Ok(provider) = RtcmProvider::from_str(s) {
            return Ok(NtripConfig::from_provider(provider));
        }

        // Strip protocol if present
        let proto = if s.starts_with("http://") {
            "http"
        } else if s.starts_with("https://") {
            "https"
        } else if s.starts_with("ntrip://") {
            "ntrip"
        } else {
            "unknown"
        };
        let s = s.trim_start_matches(&format!("{proto}://"));

        // Split host and port
        let parts: Vec<&str> = s.split(':').collect();
        if parts.is_empty() {
            return Err(NtripClientError::InvalidUrl);
        }
        let host = parts[0].to_string();

        // Parse port or use default
        let port = if parts.len() > 1 {
            parts[1]
                .parse::<u16>()
                .map_err(|_| NtripClientError::InvalidPort)?
        } else if proto == "https" {
            443
        } else {
            2101
        };
        Ok(NtripConfig {
            host,
            port,
            use_tls: port == 443,
        })
    }
}
