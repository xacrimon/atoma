mod epoch;
mod global;
mod local;
mod shield;

pub use local::Local;
pub use shield::{CowShield, Shield, ThinShield};

use global::Global;
use std::sync::Arc;

/// The `Collector` acts like the central bookkeeper, it stores all the retired functions that are queued
/// for execution along with information on what each participant is doing, Participants are pretty much always
/// thread specific as of now but cross-thread participants may be added in the future. This information can be used to determine approximately
/// when a participant last was in in a critical section and relevant shield history. The collector
/// uses this information to determine when it is safe to execute a retired function.
pub struct Collector {
    global: Arc<Global>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            global: Arc::new(Global::new()),
        }
    }

    /// Creates a shield on the appropriate local given the current thread.
    pub fn thin_shield(&self) -> ThinShield<'_> {
        Global::thin_shield(&self.global)
    }

    /// Get the local for the current thread.
    pub fn local(&self) -> Local {
        Global::local(&self.global)
    }

    /// Attempt to advance the epoch and collect garbage.
    /// The result represents whether or not the attempt to advance the global epoch
    /// was successful and if it was the integer is how many retired functions were executed.
    pub fn try_collect_light(&self) -> Result<usize, ()> {
        Global::try_collect_light(&self.global)
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}
