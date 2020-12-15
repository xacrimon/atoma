#![no_std]
#![no_main]
#![feature(lang_items, start)]

use core::panic::PanicInfo;
use flize::{Collector, tls2::{ThreadId, TlsProvider}, alloc::{VirtualAllocRef, AllocRef, Layout}};

#[lang = "start"]
fn start() -> ! {
    let collector = Collector::with_allocator_and_tls_provider();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
