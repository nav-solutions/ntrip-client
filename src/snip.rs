use std::str::FromStr;

use geoutils::Location;
use isocountry::CountryCode;
use strum::{Display, EnumString, VariantNames};
use tracing::debug;

/// Information about an NTRIP / SNIP server and its mounts
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerInfo {
    pub server: Option<String>,
    // TODO: parse this out to a date?
    pub date: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,

    pub services: Vec<MountInfo>,
}

/// Information about a specific NTRIP mount point
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MountInfo {
    pub name: String,
    pub details: String,
    pub protocol: Protocol,
    pub messages: Vec<String>,
    pub constellations: Vec<Constellation>,
    pub network: Network,
    pub country: Option<CountryCode>,
    pub location: Location,
}

/// NTRIP protocol types
#[derive(Clone, PartialEq, Debug, EnumString, Display, VariantNames)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Protocol {
    #[strum(serialize = "RTCM 3")]
    Rtcm3,
    #[strum(serialize = "RTCM 3.0")]
    Rtcm3_0,
    #[strum(serialize = "RTCM 3.2")]
    Rtcm3_2,
    #[strum(serialize = "RTCM 3.3")]
    Rtcm3_3,
    #[strum(serialize = "RAW")]
    Raw,
    #[strum(serialize = "CMRx")]
    CMRx,
    #[strum(serialize = "UNKNOWN")]
    Unknown,
}

/// NTRIP network types
#[derive(Clone, PartialEq, Debug, EnumString, Display, VariantNames)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Network {
    #[strum(serialize = "SNIP")]
    Snip,
    #[strum(serialize = "UNKNOWN")]
    Unknown,
}

/// GNSS Constellation types
#[derive(Clone, PartialEq, Debug, EnumString, Display, VariantNames)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Constellation {
    #[strum(serialize = "GPS")]
    Gps,
    #[strum(serialize = "GLO")]
    Glonass,
    #[strum(serialize = "GAL")]
    Galileo,
    #[strum(serialize = "BDS")]
    BeiDou,
    #[strum(serialize = "UNKNOWN")]
    Unknown,
}

impl ServerInfo {
    /// Parse SNIP server info from an iterator of lines
    pub fn parse<'a>(lines: impl Iterator<Item = &'a str>) -> Self {
        let mut server = None;
        let mut date = None;
        let mut content_type = None;
        let mut content_length = None;
        let mut services = Vec::new();

        for line in lines {
            if line.starts_with("Server: ") {
                server = Some(line.trim_start_matches("Server: ").to_string());
            } else if line.starts_with("Date: ") {
                date = Some(line.trim_start_matches("Date: ").to_string());
            } else if line.starts_with("Content-Type: ") {
                content_type = Some(line.trim_start_matches("Content-Type: ").to_string());
            } else if line.starts_with("Content-Length: ") {
                content_length =
                    Some(line.trim_start_matches("Content-Length: ").parse().ok()).flatten();
            } else if line.starts_with("STR;") {
                match MountInfo::parse(line) {
                    Some(info) => {
                        services.push(info);
                    },
                    None => {
                        debug!("Failed to parse STR line: {}", line);
                    },
                }
            }
        }

        ServerInfo {
            server,
            date,
            content_type,
            content_length,
            services,
        }
    }

    /// Find the nearest mount point to a given location
    pub fn find_nearest(&self, location: &Location) -> Option<(&MountInfo, f64)> {
        // If they're more than 100km away, we don't want to know
        let mut min_distance = 100_000f64;
        let mut min_entry = None;

        for (i, s) in self.services.iter().enumerate() {
            if let Ok(d) = s.location.distance_to(location) {
                debug!("Distance to {}: {:.3} km", s.name, d);
                if d.meters() < min_distance {
                    min_distance = d.meters();
                    min_entry = Some(i);
                }
            }
        }

        min_entry.map(|i| (&self.services[i], min_distance))
    }
}

impl MountInfo {
    pub fn parse(info: &str) -> Option<Self> {
        let parts: Vec<&str> = info.split(';').collect();
        if parts.len() < 2 {
            return None;
        }

        if parts[0] != "STR" {
            return None;
        }

        let name = parts[1].to_string();
        let details = parts[2].trim().to_string();
        let protocol = parts
            .get(3)
            .and_then(|s| Protocol::from_str(s).ok())
            .unwrap_or(Protocol::Raw);

        let messages = match parts.get(4) {
            Some(msgs) => msgs.split(",").map(|m| m.trim().to_string()).collect(),
            None => vec![],
        };

        // What is part 5?

        // Part 6: constellations
        let constellations = match parts.get(6) {
            Some(c) => c
                .split('+')
                .map(|s| {
                    Constellation::from_str(s)
                        .ok()
                        .unwrap_or(Constellation::Unknown)
                })
                .collect::<Vec<_>>(),
            None => vec![],
        };

        // Part 7: network
        let network = parts
            .get(7)
            .and_then(|s| Network::from_str(s).ok())
            .unwrap_or(Network::Unknown);

        // Part 8: country
        let country = parts.get(8).and_then(|s| CountryCode::for_alpha3(s).ok());

        // Parts 9-11: lat, lon, (alt?)
        let location = Location::new(
            parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            parts.get(10).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        );

        // TODO: the rest of the fields

        Some(MountInfo {
            name,
            details,
            protocol,
            messages,
            constellations,
            network,
            country,
            location,
        })
    }
}

#[cfg(test)]
mod tests {
    use http::Method;
    use tracing::{debug, info, trace};

    use super::*;

    fn setup_logging() {
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .compact()
            .without_time()
            .with_max_level(tracing::level_filters::LevelFilter::DEBUG)
            .try_init();
    }

    #[test]
    fn test_parse_server_info() {
        setup_logging();

        let info = "STR;VargaRTKhr;Is near: Zagreb, Zagreb;RTCM 3.2;1006(1),1033(1),1074(1),1084(1),1094(1),1124(1),1230(1);;GPS+GLO+GAL+BDS;SNIP;HRV;46.44;16.50;1;0;sNTRIP;none;B;N;0;\n";

        let server_info = MountInfo::parse(info).unwrap();

        assert_eq!(server_info.name, "VargaRTKhr");
        assert_eq!(server_info.details, "Is near: Zagreb, Zagreb");
        assert_eq!(server_info.protocol, Protocol::Rtcm3_2);
        assert_eq!(
            server_info.messages,
            vec!["1006(1)", "1033(1)", "1074(1)", "1084(1)", "1094(1)", "1124(1)", "1230(1)"]
        );
        assert_eq!(
            server_info.constellations,
            vec![
                Constellation::Gps,
                Constellation::Glonass,
                Constellation::Galileo,
                Constellation::BeiDou
            ]
        );
        assert_eq!(server_info.network, Network::Snip);
        assert_eq!(
            server_info.country,
            Some(CountryCode::for_alpha3("HRV").unwrap())
        );
        assert!((server_info.location.latitude() - 46.44).abs() < 0.001);
        assert!((server_info.location.longitude() - 16.50).abs() < 0.001);
    }

    #[test]
    fn test_parse_snip_info() {
        setup_logging();

        let snip_response = "
            SOURCETABLE 200 OK\n
            Server: NTRIP SNIP/2.0\n
            Date: Wed, 26 Jun 2024 12:00:00 GMT\n
            Content-Type: text/plain; charset=utf-8\n
            Content-Length: 1234\n
            STR;warrakam;Is near: Sydney, New South Wales;RTCM 3;1004(1), 1005(10), 1008(10), 1012(1), 1019(2), 1020(2), 1033(10), 1042(2), 1046(2), 1077(1), 1087(1), 1097(1), 1127(1), 1230(30);2;;SNIP;AUS;-36.37;144.46;1;0;SNIP;none;B;N;11740;\n
            STR;VargaRTKhr;Is near: Zagreb, Zagreb;RTCM 3.2;1006(1),1033(1),1074(1),1084(1),1094(1),1124(1),1230(1);;GPS+GLO+GAL+BDS;SNIP;HRV;46.44;16.50;1;0;sNTRIP;none;B;N;0;\n
        ";

        let lines = snip_response
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<&str>>();

        debug!("Lines: {:?}", &lines[..10]);

        let snip_info = ServerInfo::parse(lines.iter().cloned());

        debug!("SNIP Info: {:#?}", snip_info);
    }

    #[tokio::test]
    #[ignore = "Requires network access"]
    async fn test_ntrip_rtk2go() {
        setup_logging();

        let client = reqwest::Client::builder()
            .http1_ignore_invalid_headers_in_responses(true)
            .http09_responses()
            .user_agent(format!(
                "NTRIP {}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap();

        let req = client
            .request(Method::GET, "http://rtk2go.com:2101")
            .header("Ntrip-Version", "Ntrip/2.0")
            .build()
            .unwrap();

        let res = client.execute(req).await.expect("Fetch failed");

        info!("Fetched NTRIP response: {:?}", res.status());

        assert!(res.status().is_success());

        let body = res.text().await.unwrap();

        let lines = body.lines().collect::<Vec<&str>>();

        trace!("Lines: {:?}", &lines[..10]);

        let snip_info = ServerInfo::parse(lines.iter().cloned());

        trace!("SNIP Info: {:#?}", snip_info);
    }
}
