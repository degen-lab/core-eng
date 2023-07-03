# Welcome to the Trust Machines Core Eng Team's Mono Repo

We use this repository to rapidly incubate and iterate on new ideas related Bitcoin or Bitcoin Layers such as Stacks.
The primary focus of the team for 2023 is sBTC.

What is sBTC? https://github.com/stacks-network/stacks-blockchain/wiki/sBTC-Eng-Wiki

[![rust](https://github.com/Trust-Machines/core-eng/actions/workflows/rust.yml/badge.svg)](https://github.com/Trust-Machines/core-eng/actions/workflows/rust.yml)
[![clarity](https://github.com/Trust-Machines/core-eng/actions/workflows/clarinet.yml/badge.svg)](https://github.com/Trust-Machines/core-eng/actions/workflows/clarinet.yml)
[![CodeFactor](https://www.codefactor.io/repository/github/trust-machines/stacks-sbtc/badge)](https://www.codefactor.io/repository/github/trust-machines/stacks-sbtc)

Documentation: https://trust-machines.github.io/stacks-sbtc

## Projects

- [relay-server](./relay-server/) is a simple HTTP relay server.
- [stacks-signer-api](./stacks-signer-api) is an API server for interacting with a Stacks signer binary.

## Prerequisites

- [Rust 1.67.1+](https://www.rust-lang.org).
- [Deno 1.30.3+](https://deno.land).
- Bitcoin Core 22.0. Run [./.install/bitcoin.sh](./.install/bitcoin.sh).
