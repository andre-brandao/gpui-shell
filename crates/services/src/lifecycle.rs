//! Uniform start/stop lifecycle for services with background work.
//!
//! Every service owns its state as `Mutable<T>` handed out through
//! `subscribe()`, and widgets hold those signals for the lifetime of the
//! window. Restarting a service therefore cannot replace the service value -
//! that would orphan every existing subscriber. Instead the *run* is
//! replaced: [`Lifecycle::begin`] hands the listener a [`RunToken`] tied to a
//! generation counter, and stopping or restarting bumps that counter so the
//! previous listener sees `alive() == false` and bows out.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_signals::signal::{Mutable, MutableSignalCloned};
use serde::{Deserialize, Serialize};

use crate::ServiceStatus;

/// How a service is brought up at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceMode {
    /// Started during application startup.
    #[default]
    Eager,
    /// Started on first use.
    Lazy,
    /// Never started.
    Off,
}

impl ServiceMode {
    pub const ALL: [ServiceMode; 3] = [ServiceMode::Eager, ServiceMode::Lazy, ServiceMode::Off];

    pub fn label(&self) -> &'static str {
        match self {
            ServiceMode::Eager => "eager",
            ServiceMode::Lazy => "lazy",
            ServiceMode::Off => "off",
        }
    }
}

/// Shared status + run generation for one service.
#[derive(Debug, Clone)]
pub struct Lifecycle {
    status: Mutable<ServiceStatus>,
    mode: Mutable<ServiceMode>,
    /// Incremented on every start and stop. A listener whose token no longer
    /// matches has been superseded and must exit.
    generation: Arc<AtomicU64>,
}

impl Lifecycle {
    pub fn new(mode: ServiceMode) -> Self {
        Self {
            status: Mutable::new(ServiceStatus::Stopped),
            mode: Mutable::new(mode),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }

    /// Signal that emits on every status change.
    pub fn status_signal(&self) -> MutableSignalCloned<ServiceStatus> {
        self.status.signal_cloned()
    }

    pub fn mode(&self) -> ServiceMode {
        self.mode.get()
    }

    pub fn set_mode(&self, mode: ServiceMode) {
        self.mode.set_neq(mode);
    }

    /// True while the service is starting or running.
    pub fn is_up(&self) -> bool {
        !matches!(
            &*self.status.lock_ref(),
            ServiceStatus::Stopped | ServiceStatus::Unavailable
        )
    }

    /// True when the service has never run or was stopped on request. An
    /// errored service is *not* stopped: it must not be restarted behind the
    /// user's back on every access.
    pub fn is_stopped(&self) -> bool {
        matches!(&*self.status.lock_ref(), ServiceStatus::Stopped)
    }

    /// Start a new run, invalidating any listener from a previous one.
    ///
    /// The returned token carries the status handle, so listeners report
    /// through it and stale runs are silently ignored.
    pub fn begin(&self) -> RunToken {
        let id = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let token = RunToken {
            status: self.status.clone(),
            generation: self.generation.clone(),
            id,
        };
        token.set(ServiceStatus::Initializing);
        token
    }

    /// Stop the current run and mark the service stopped.
    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.status.set_neq(ServiceStatus::Stopped);
    }

    /// Mark a service that has no work to do on this system.
    pub fn set_unavailable(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.status.set_neq(ServiceStatus::Unavailable);
    }
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new(ServiceMode::default())
    }
}

/// Handle held by a running listener.
///
/// Writes through a stale token are dropped, so a listener that only notices
/// the stop at its next loop iteration cannot resurrect a stopped service's
/// status.
#[derive(Debug, Clone)]
pub struct RunToken {
    status: Mutable<ServiceStatus>,
    generation: Arc<AtomicU64>,
    id: u64,
}

impl RunToken {
    /// False once the service was stopped or restarted: exit the listener.
    pub fn alive(&self) -> bool {
        self.generation.load(Ordering::SeqCst) == self.id
    }

    pub fn set(&self, status: ServiceStatus) {
        if self.alive() {
            self.status.set_neq(status);
        }
    }

    pub fn active(&self) {
        self.set(ServiceStatus::Active);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.set(ServiceStatus::Error(Some(message.into())));
    }
}

/// A service whose background work can be inspected and controlled.
pub trait ManagedService: Send + Sync {
    /// Display name, also the key used in configuration.
    fn name(&self) -> &'static str;

    fn lifecycle(&self) -> &Lifecycle;

    /// Bytes of state this service retains.
    ///
    /// Services share the shell's heap, so this is the size of what the
    /// service keeps alive, not a resident-set measurement.
    fn memory_bytes(&self) -> usize;

    /// (Re)start background work, replacing any current run.
    fn start(&self);

    /// Services the shell cannot run without report `false` and are shown
    /// read-only.
    fn controllable(&self) -> bool {
        true
    }

    fn status(&self) -> ServiceStatus {
        self.lifecycle().status()
    }

    fn status_signal(&self) -> MutableSignalCloned<ServiceStatus> {
        self.lifecycle().status_signal()
    }

    fn mode(&self) -> ServiceMode {
        self.lifecycle().mode()
    }

    fn stop(&self) {
        self.lifecycle().stop();
    }

    fn restart(&self) {
        self.start();
    }

    /// Start a lazy service on first use. Cheap enough to call per access.
    fn ensure_started(&self) {
        if self.lifecycle().mode() == ServiceMode::Lazy && self.lifecycle().is_stopped() {
            self.start();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_and_restart_invalidate_the_previous_run() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);

        let first = lifecycle.begin();
        first.active();
        assert!(first.alive());
        assert_eq!(lifecycle.status(), ServiceStatus::Active);

        // A restart supersedes the first run.
        let second = lifecycle.begin();
        assert!(!first.alive());
        assert!(second.alive());

        // The superseded listener can no longer publish status.
        first.error("stale");
        assert_eq!(lifecycle.status(), ServiceStatus::Initializing);

        second.active();
        lifecycle.stop();
        assert!(!second.alive());
        assert_eq!(lifecycle.status(), ServiceStatus::Stopped);
        assert!(!lifecycle.is_up());

        second.active();
        assert_eq!(lifecycle.status(), ServiceStatus::Stopped);
    }
}
