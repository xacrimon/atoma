#![no_std]
#![no_main]
#![feature(lang_items, start)]

use core::panic::PanicInfo;

#[lang = "start"]
fn start() -> ! {
    loop {}
}

#[lang = "eh_personality"]
fn eh_personality() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
