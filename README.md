# flize

flize implements epoch-based reclamation with less restrictions than `crossbeam-epoch`.

A primary goal of this crate so to have a very rusty API and
to have clear and simple source code.

We have an MSRV of 1.36 and increasing it is considered a breaking change.

Furthermore as made evident in the testing benchmarks flize is significantly
less vulnerable hitting cases of unoptimized behaviour and unacceptable usage of memory than crossbeam-epoch.
In our testing flize is much more consistent in resource usage than crossbeam-epoch.

This crate is useful if you have resources that require destruction
in a concurrent environment and you don't want to pay the price of locking.

[![version](https://img.shields.io/crates/v/flize)](https://crates.io/crates/flize)

[![documentation](https://docs.rs/flize/badge.svg)](https://docs.rs/flize)

[![downloads](https://img.shields.io/crates/d/flize)](https://crates.io/crates/flize)

[![minimum rustc version](https://img.shields.io/badge/rustc-1.36+-orange.svg)](https://crates.io/crates/flize)

## Testing

Testing is done automatically by our CI on every push and pull request.
Releases and the master branch should always pass tests.

Due to the nature of this crate it is heavily architecture and OS dependent.
Because of this we run tests on a number of different architectures using emulation
and check that the crate builds successfully on Linux, Windows and macOS.

All testing is done on our MSRV toolchain which is `stable-2019-07-04`.

## Test targets

These are targets we build and test on and the OS used for it.

### Ubuntu 18.04

- `x86_64-unknown-linux-gnu`
- `i686-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `powerpc64le-unknown-linux-gnu`

### Windows Server 2019

- `x86_64-pc-windows-msvc`

### macOS Catalina 10.15

- `x86_64-apple-darwin`
