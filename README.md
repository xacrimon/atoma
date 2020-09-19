# flize

flize implements epoch-based reclamation with less restrictions than `crossbeam-epoch`.

A primary goal of this crate so to have a very rusty API and
to have clear and simple source code.

Please note that there are still many performance optimizations we have not implemented yet.

Furthermore as made evident in the testing benchmarks flize is significantly
less vulnerable hitting cases of unoptimized behaviour and unacceptable usage of memory than crossbeam-epoch.
In our testing flize is much more consistent in resource usage than crossbeam-epoch.

This crate is useful if you have resources that require destruction
in a concurrent environment and you don't want to pay the price of locking.

[![version](https://img.shields.io/crates/v/flize)](https://crates.io/crates/flize)

[![documentation](https://docs.rs/flize/badge.svg)](https://docs.rs/flize)

[![downloads](https://img.shields.io/crates/d/flize)](https://crates.io/crates/flize)

[![minimum rustc version](https://img.shields.io/badge/rustc-1.36+-orange.svg)](https://crates.io/crates/flize)
