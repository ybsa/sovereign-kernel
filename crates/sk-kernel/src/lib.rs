//! Based on Sovereign Kernel's Sovereign Kernel-kernel.

#[macro_export]
macro_rules! rlock {
    ($lock:expr) => {
        $lock
            .read()
            .expect(concat!("RwLock poisoned at ", file!(), ":", line!()))
    };
}

#[macro_export]
macro_rules! wlock {
    ($lock:expr) => {
        $lock.write().expect(concat!(
            "RwLock write-lock poisoned at ",
            file!(),
            ":",
            line!()
        ))
    };
}

#[macro_export]
macro_rules! lock {
    ($lock:expr) => {
        $lock
            .lock()
            .expect(concat!("Mutex lock poisoned at ", file!(), ":", line!()))
    };
}

pub mod approval;
pub mod audit;
pub mod auth;
pub mod auto_reply;
pub mod background;
pub mod bus;
pub mod capabilities;
pub mod config;
pub mod config_reload;
pub mod cron;
pub mod error;
pub mod event_bus;
pub mod executor;
pub mod heartbeat;
pub mod kernel;
pub mod metering;
pub mod pairing;
pub mod registry;
pub mod scheduler;
pub mod supervisor;
pub mod tools;
pub mod triggers;
pub mod wizard;
pub mod workflow;

pub use kernel::SovereignKernel;
