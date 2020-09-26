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

    pub fn shield(&self) -> Shield<'_> {
        Global::shield(&self.global)
    }

    pub fn local(&self) -> Local {
        Global::local(&self.global)
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}
