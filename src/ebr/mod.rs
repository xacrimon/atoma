mod epoch;
mod global;
mod local;
mod shield;

pub use local::Local;
pub use shield::{CowShield, Shield};

use global::Global;
use std::sync::Arc;

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
    pub fn shield(&self) -> Shield<'_> {
        Global::shield(&self.global)
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
