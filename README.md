NTRIP Client
============

[![Rust](https://github.com/nav-solutions/ntrip-client/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/ntrip-client/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/ntrip-client/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/ntrip-client/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/ntrip-client/badge.svg)](https://docs.rs/ntrip-client/)
[![crates.io](https://img.shields.io/crates/d/ntrip-client.svg)](https://crates.io/crates/ntrip-client)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/ntrip-client/blob/main/LICENSE)

NTRIP client used by all our applications that require RTCM messaging (downlink), through NTRIP connection.

Backend framework
=================

`ntrip-client` currently uses `tokio` as the multi-threading backend.

Getting started
===============

Refer to the [provided example](examples/) for a complete demo.

```rust
// Configure server
let ntrip_config = "centipede".parse::<NtripConfig>();
let ntrip_creds = NtripCredentials{
    user: "centipede".to_string(),
    pass: "centipede".to_string(),
}

// Setup client
let mut client = NtripClient::new(ntrip_config, ntrip_creds).await.unwrap();

// List mounts
let server_info = client.list_mounts().await.unwrap();
for m in server_info.mounts {
    println!("{} - {}", m.name, m.details);
}

// Subscribe to a mount
let (exit_tx, exit_rx) = tokio::sync::broadcast(1);
let handle = client.mount("VALDM", exit_tx.clone());

loop {
    select!{
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
```

Licensing
=========

This library is part of the [NAV-solutions framework](https://github.com/nav-solutions) 
which is licensed under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.
