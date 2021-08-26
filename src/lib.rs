#![cfg_attr(not(feature = "std"), no_std)]

//! flize is an implementation of epoch-based lock-free resource reclamation.
//! The core is based around the paper "Practical lock-freedom" by Keir Fraser
//! although many modifications have been made to adapt the scheme to perform well on
//! on modern hardware and scale to a high degree.
//!
//! This crate is useful if you have resources that require destruction in a
//! concurrent environment and you don't want to pay the price of locking.
//!
//! flize attempts to encourage simplicity and avoid implicit global state.
//! Additionally we try to reduce the amount of hidden gotchas and ways to shoot yourself in the foot.
//!
//! A basic understanding of the previously mentioned paper and lock-free memory management
//! is assumed throughout this documentation. If it isn't clearly explained here
//! it's probably well defined in the paper. If not, please submit an issue and we will try to add documentation.
//!
//! The core workflow of this library involves a couple of different types.
//! First up you've got `Collector` and `Shield`, they are the gateway to interacting with the core functionality.
//! A collector keeps track of what threads are reading protected pointers and which aren't. It does this
//! by requiring allowing the user to create `Shield`s which act as as sort of guard. The creation
//! and destruction of this type interacts with the internal bookkeeping in the `Collector`.
//! The existance of at least one `Shield` implies that the thread is in a critical section
//! and may access protected pointers.
//!
//! Shields are in turn needed to load `Atomic`s which simply boil down to an atomic pointer.
//! Upon loading an `Atomic` you get a `Shared` which is a pointer type bounded by the lifetime of the shield.
//! This is an attempt to prevent you from using a pointer without being in a critical section.
//! `Atomic` implements all the common atomic operations for reading and modification such as `load`, `store` and `compare_and_swap`.
//! When you remove all pointers to an object from shared memory it will be safe to destroy
//! after all threads that could possibly have a reference have exited their critical sections (dropped all their shields).
//! This can be accomplished by calling `Shield::defer` and supplying a closure. This closure
//! will then be executed once all threads currently in a critical section have exited it at least once.
//!
//! flize also handles a couple of other things that vastly improves quality of life
//! and simplicity for users compared to `crossbeam-epoch`. For example flize provides you
//! with full support for low and high bit pointer tags. It takes this one step further
//! and allows for arbitrary structs that can be serialized to an array of bits to
//! be used as tags. The library will handle reading, writing and stripping these tags from and to pointers
//! along with serialization for you with a set of helper methods on `Shared` for interacting with tags ergonomically.
//!
//! By default we attempt to utilize OS memory barriers to improve bookkeeping performance
//! on Windows, Linux and macOS. For other targets we fall back to a more general although slower implementation.
//! To make this possible we conditionally depend on `winapi` on Windows targets and `libc` on Linux and macOS targets.
//! This accelerated bookkeeping is controlled by the `fast-barrier` Cargo feature.
//! This flag is enabled by default and disabling it will cause the more general implementation to be compiled on all targets.

pub mod alloc;
mod atomic;
mod backoff;
mod barrier;
mod cache_padded;
mod deferred;
mod ebr;
mod heap;
mod lazy;
mod mutex;
mod queue;
mod shared;
mod tag;
pub mod tls2;

pub use atomic::Atomic;
pub use backoff::Backoff;
pub use cache_padded::CachePadded;
pub use ebr::{
    unprotected, Collector, CowShield, FullShield, Local, Shield, ThinShield, UnprotectedShield,
};
pub use shared::Shared;
pub use tag::{NullTag, Tag};
