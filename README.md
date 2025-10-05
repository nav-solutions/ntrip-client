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

`ntrip-client` uses `tokio` as the socket, channel and threading backend.

Getting started
===============

Refer to the online documentation and [the provided examples](examples/).

Licensing
=========

This library is part of the [NAV-solutions framework](https://github.com/nav-solutions) 
which is licensed under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.
