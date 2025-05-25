NTRIP Client
============

[![Rust](https://github.com/rtk-rs/ntrip-client/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/ntrip-client/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/ntrip-client/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/ntrip-client/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/ntrip-client/badge.svg)](https://docs.rs/ntrip-client/)
[![crates.io](https://img.shields.io/crates/d/ntrip-client.svg)](https://crates.io/crates/ntrip-client)

[![MRSV](https://img.shields.io/badge/MSRV-1.78.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.78.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/ntrip-client/blob/main/LICENSE)

NTRIP client used by all our applications that require RTCM messaging (downlink), through NTRIP connection.

Backend framework
=================

`ntrip-client` currently uses `tokio` as the multi-threading backend.

Getting started
===============

```toml
ntrip-client = "0.1"
```

```rust
let mut client = NTRIPClient::new("caster.centipede.fr", 2101, "ENSMM")
    .with_credentials("centipede", "centipede");

// deploy using 'tokio' framework
client.run()
    .await
    .unwrap_or_else(|e| {
        panic!("Failed to deploy NTRIP client: {}", e);
    });
```

Licensing
=========

This library is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.
