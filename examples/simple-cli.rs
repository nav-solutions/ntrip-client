use clap::Parser;
use futures::StreamExt;
use geoutils::Location;
use ntrip_client::{
    config::{NtripConfig, NtripCredentials},
    NtripClient,
};
use tokio::select;
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Subscriber as FmtSubscriber, EnvFilter};

/// NTRIP command line tool
#[derive(Clone, PartialEq, Debug, Parser)]
struct Args {
    #[clap()]
    /// NTRIP server identifier or URI ("rtk2go", "linz" etc., or "[ntrip|http|https]://host:port")
    pub ntrip_host: NtripConfig,

    #[clap(flatten)]
    pub ntrip_creds: NtripCredentials,

    #[clap(subcommand)]
    pub command: Commands,

    #[clap(long, default_value = "info")]
    /// Set log level
    pub log_level: LevelFilter,
}

#[derive(Clone, PartialEq, Debug, Parser)]
pub enum Commands {
    /// List mount points on an NTRIP server
    List,
    /// Find the nearest mount point to a specified location
    FindNearest {
        #[clap()]
        lat: f64,
        #[clap()]
        lon: f64,
    },
    /// Subscribe to a specified mount point and print received RTCM messages
    Subscribe {
        #[clap()]
        mount: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    let filter = EnvFilter::from_default_env().add_directive(args.log_level.into());
    let _ = FmtSubscriber::builder()
        .compact()
        .without_time()
        .with_max_level(args.log_level)
        .with_env_filter(filter)
        .try_init();

    info!("Start NTRIP/RTMP tool");

    debug!("Args {args:?}");

    // Setup interrupt / exit handler
    let (exit_tx, mut exit_rx) = tokio::sync::broadcast::channel(1);
    let e = exit_tx.clone();
    tokio::task::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Received Ctrl-C, shutting down...");
        e.send(()).unwrap();
    });

    let mut client = NtripClient::new(args.ntrip_host.clone(), args.ntrip_creds.clone()).await?;

    match args.command {
        Commands::List => {
            // List available NTRIP mounts using SNIP
            info!("Listing NTRIP mounts");

            let info = client.list_mounts().await.unwrap();

            for s in info.services {
                info!(
                    "{} - {} ({:.3}, {:.3})",
                    s.name,
                    s.details,
                    s.location.latitude(),
                    s.location.longitude()
                );
            }
        },
        Commands::FindNearest { lat, lon } => {
            // Find the nearest NTRIP mount to the specified location
            info!("Finding nearest NTRIP mount to ({}, {})", lat, lon);

            let info = client.list_mounts().await.unwrap();

            let target_location = Location::new(lat, lon);

            match info.find_nearest(&target_location) {
                Some((s, d)) => {
                    info!(
                        "Nearest mount: {} - {} ({:.3}, {:.3}), {:.3} km away",
                        s.name,
                        s.details,
                        s.location.latitude(),
                        s.location.longitude(),
                        d / 1000.0
                    );
                },
                None => {
                    info!("No mounts found");
                },
            }
        },
        Commands::Subscribe { mount } => {
            // Subscribe to the specified NTRIP mount
            debug!("Connecting to NTRIP server");

            // Setup the NTRIP client
            let mut client = client.mount(mount, exit_tx.clone()).await?;

            // Process incoming RTCM messages
            loop {
                select! {
                    m = client.next() => match m {
                        Some(m) => {
                            info!("Received RTCM message: {:?}", m);
                        },
                        None => {
                            error!("NTRIP client stream ended");
                            break;
                        }
                    },
                    _ = exit_rx.recv() => {
                        info!("Exiting on signal");
                        break;
                    }
                }
            }
        },
    }

    debug!("Exiting");

    Ok(())
}
