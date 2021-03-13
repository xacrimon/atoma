mod bag;
mod ct;
mod epoch;
mod global;
mod local;
mod shield;

pub use local::Local;
pub use shield::{unprotected, CowShield, FullShield, Shield, ThinShield, UnprotectedShield};

use crate::alloc::AllocRef;
use crate::heap::Arc;
use crate::tls2::TlsProvider;
use core::fmt;
use global::Global;

#[cfg(feature = "std")]
use crate::tls2::std_tls_provider;

#[cfg(feature = "std")]
use crate::alloc::GlobalAllocator;

const MAX_GARBAGE_BYTES: usize = 1024 * 1024;
const ADVANCE_PROBABILITY: usize = 256;

/// The `Collector` acts like the central bookkeeper, it stores all the retired functions that are queued
/// for execution along with information on what each participant is doing, Participants are pretty much always
/// thread specific as of now but cross-thread participants may be added in the future. This information can be used to determine approximately
/// when a participant last was in in a critical section and relevant shield history. The collector
/// uses this information to determine when it is safe to execute a retired function.
pub struct Collector {
    global: Arc<Global>,
}

impl Collector {
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        let allocator = AllocRef::new(GlobalAllocator);
        let tls_provider = std_tls_provider();
        Self::with_allocator_and_tls_provider(allocator, tls_provider)
    }

    pub fn with_allocator_and_tls_provider(
        allocator: AllocRef,
        tls_provider: &'static dyn TlsProvider,
    ) -> Self {
        Self {
            global: Arc::new(
                Global::new(allocator.clone(), tls_provider, MAX_GARBAGE_BYTES),
                allocator,
            ),
        }
    }

    /// Creates a shield on the appropriate local given the current thread.
    pub fn thin_shield(&self) -> ThinShield<'_> {
        Global::thin_shield(&self.global)
    }

    pub fn full_shield(&self) -> FullShield<'_> {
        Global::full_shield(&self.global)
    }

    /// Get the local for the current thread.
    pub fn local(&self) -> Local {
        Global::local(&self.global)
    }

    /// Attempt to advance the epoch and collect garbage.
    /// Returns true if the epoch was cycled and garbage collected.
    pub fn try_collect_light(&self) -> bool {
        Global::try_collect_light(&self.global)
    }
}

#[cfg(feature = "std")]
impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Collector {}
unsafe impl Sync for Collector {}

impl fmt::Debug for Collector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Collector { .. }")
    }
}
