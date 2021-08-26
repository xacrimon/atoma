# flize

flize implements fast non-global epoch-based reclamation

flize strives for excellent performance achieved through relentless optimization with a clean and rusty API.

We have an MSRV of 1.51 and increasing it is considered a breaking change.

This crate is useful if you have resources that require destruction
in a concurrent environment and you don't want to pay the price of locking.

[![version](https://img.shields.io/crates/v/flize)](https://crates.io/crates/flize)

[![documentation](https://docs.rs/flize/badge.svg)](https://docs.rs/flize)

[![downloads](https://img.shields.io/crates/d/flize)](https://crates.io/crates/flize)

[![minimum rustc version](https://img.shields.io/badge/rustc-1.51+-orange.svg)](https://crates.io/crates/flize)

## Features

- `std` - Enables the default TLS and allocator providers that hook into the standard library.
- `fast-barrier` - Accelerates shield creation and destruction significantly on Windows, macOS and Linux and
falls back to a platform agnostic implementation on other targets.

Both are enabled by default.

## no_std

Flize supports `no_std` with or without `alloc` by disabling the `std` feature.

## Examples

The `examples` directory contains some sample code using flize.
We're always keen to add more examples so please feel free to expand
the collection if you've got a good implementation of something relatively simple.

## Testing

Testing is done automatically by our CI on every push and pull request.
Releases and the master branch should always pass tests.

Due to the nature of this crate it is heavily architecture and OS dependent.
Because of this we run tests on a number of different architectures using emulation
and check that the crate builds successfully on Linux, Windows and macOS.

All testing is done on our MSRV toolchain which is Rust 1.51 `stable-2021-03-25`.

### Test targets

These are targets we build and test on.

#### Ubuntu 18.04

- `x86_64-unknown-linux-gnu`
- `i686-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `powerpc64le-unknown-linux-gnu`

#### Windows Server 2019

- `x86_64-pc-windows-msvc`

#### macOS Catalina 10.15

- `x86_64-apple-darwin`
