//! This module implements efficient barriers for fast and slow paths.
//! Often you have the ability to sacrifice performance in the slow path for performance in the fast path
//! and this is often something you want to do.
//!
//! Here we have two kinds of barriers, light and heavy ones respectively.
//! They can take advantage of OS provided functionality for process wide memory barriers
//! which means the ability to skip barriers in certain fast paths.
//!
//! This module will conditionally compile an optimized implementation depending on the target OS
//! and will attempt to determine the most efficient setup dynamically at runtime.
//!
//! When no specialized implementation is available we fall back to executing a normal
//! sequentially consistent barrier in both the light and heavy barriers.

#[cfg(all(target_os = "windows"))]
pub use windows::{light_barrier, strong_barrier};

#[cfg(all(target_os = "linux"))]
pub use linux::{light_barrier, strong_barrier};

#[cfg(all(target_os = "macos"))]
pub use macos::{light_barrier, strong_barrier};

#[cfg(any(
    all(
        not(target_os = "linux"),
        not(target_os = "windows"),
        not(target_os = "macos")
    )
))]
pub use fallback::{light_barrier, strong_barrier};

#[cfg(all(target_os = "windows"))]
mod windows {
    use core::sync::atomic::{compiler_fence, Ordering};
    use winapi::um::processthreadsapi;

    pub fn strong_barrier() {
        unsafe {
            processthreadsapi::FlushProcessWriteBuffers();
        }
    }

    pub fn light_barrier() {
        compiler_fence(Ordering::Release);
    }
}

#[cfg(all(target_os = "linux"))]
mod linux {
    use crate::lazy::Lazy;
    use core::sync::atomic::{compiler_fence, fence, Ordering};

    pub fn strong_barrier() {
        match STRATEGY.get() {
            Strategy::Membarrier => membarrier::barrier(),
            Strategy::Fallback => fence(Ordering::SeqCst),
        }
    }

    pub fn light_barrier() {
        match STRATEGY.get() {
            Strategy::Membarrier => compiler_fence(Ordering::Release),
            Strategy::Fallback => fence(Ordering::SeqCst),
        }
    }

    enum Strategy {
        Membarrier,
        Fallback,
    }

    static STRATEGY: Lazy<Strategy> = Lazy::new(|| {
        if membarrier::is_supported() {
            Strategy::Membarrier
        } else {
            Strategy::Fallback
        }
    });

    mod membarrier {
        #[repr(i32)]
        #[allow(dead_code, non_camel_case_types, clippy::upper_case_acronyms)]
        enum membarrier_cmd {
            MEMBARRIER_CMD_QUERY = 0,
            MEMBARRIER_CMD_GLOBAL = 1,
            MEMBARRIER_CMD_GLOBAL_EXPEDITED = 1 << 1,
            MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED = 1 << 2,
            MEMBARRIER_CMD_PRIVATE_EXPEDITED = 1 << 3,
            MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED = 1 << 4,
            MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE = 1 << 5,
            MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE = 1 << 6,
        }

        fn sys_membarrier(cmd: membarrier_cmd) -> libc::c_long {
            unsafe {
                #[allow(clippy::unnecessary_cast)]
                libc::syscall(libc::SYS_membarrier, cmd as libc::c_int, 0 as libc::c_int)
            }
        }

        pub fn is_supported() -> bool {
            // Queries which membarrier commands are supported. Checks if private expedited
            // membarrier is supported.
            let ret = sys_membarrier(membarrier_cmd::MEMBARRIER_CMD_QUERY);
            if ret < 0
                || ret & membarrier_cmd::MEMBARRIER_CMD_PRIVATE_EXPEDITED as libc::c_long == 0
                || ret & membarrier_cmd::MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED as libc::c_long
                    == 0
            {
                return false;
            }

            // Registers the current process as a user of private expedited membarrier.
            if sys_membarrier(membarrier_cmd::MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED) < 0 {
                return false;
            }

            true
        }

        macro_rules! fatal_assert {
            ($cond:expr) => {
                if !$cond {
                    #[allow(unused_unsafe)]
                    unsafe {
                        libc::abort();
                    }
                }
            };
        }

        pub fn barrier() {
            fatal_assert!(sys_membarrier(membarrier_cmd::MEMBARRIER_CMD_PRIVATE_EXPEDITED) >= 0);
        }
    }
}

#[cfg(all(target_os = "macos"))]
mod macos {
    use core::ptr::null_mut;
    use core::sync::atomic::{compiler_fence, Ordering};
    use once_cell::sync::Lazy;
    use std::sync::{Mutex, MutexGuard};

    struct Ptr(*mut libc::c_void);

    unsafe impl Send for Ptr {}
    unsafe impl Sync for Ptr {}

    static DUMMY_PAGE: Lazy<Mutex<Ptr>> = Lazy::new(|| Mutex::new(Ptr(alloc_page())));

    fn alloc_page() -> *mut libc::c_void {
        unsafe {
            let page = libc::mmap(
                null_mut(),
                1,
                libc::PROT_READ,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );

            assert!(page != libc::MAP_FAILED);
            assert!(libc::mlock(page, 1) >= 0);
            page
        }
    }

    pub fn strong_barrier() {
        unsafe {
            let dummy_page = DUMMY_PAGE.lock().unwrap();
            assert!(libc::mprotect(dummy_page.0, 1, libc::PROT_READ | libc::PROT_WRITE) >= 0);
            assert!(libc::mprotect(dummy_page.0, 1, libc::PROT_READ) >= 0);
        }
    }

    pub fn light_barrier() {
        compiler_fence(Ordering::Release);
    }
}

#[cfg(any(
    all(
        not(target_os = "linux"),
        not(target_os = "windows"),
        not(target_os = "macos")
    )
))]
mod fallback {
    use core::sync::atomic::{fence, Ordering};

    pub fn strong_barrier() {
        fence(Ordering::SeqCst);
    }

    pub fn light_barrier() {
        fence(Ordering::SeqCst);
    }
}
