# flize

flize implements epoch-based reclamation with less restrictions than `crossbeam-epoch`.

This crate is useful if you have resources that require destruction
in a concurrent environment and you don't want to pay the price of locking.

[![version](https://img.shields.io/crates/v/flize)](https://crates.io/crates/flize)

[![documentation](https://docs.rs/flize/badge.svg)](https://docs.rs/flize)

[![downloads](https://img.shields.io/crates/d/flize)](https://crates.io/crates/flize)

[![minimum rustc version](https://img.shields.io/badge/rustc-1.44.1-orange.svg)](https://crates.io/crates/flize)
