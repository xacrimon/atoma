#![no_std]
#![no_main]
#![feature(lang_items, start)]

use core::panic::PanicInfo;
use flize::Collector;

#[lang = "start"]
fn start() -> ! {
    let collector = Collector::new();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
