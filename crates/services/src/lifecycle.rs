//! Uniform start/stop lifecycle for services with background work.
//!
//! Every service owns its state as `Mutable<T>` handed out through
//! `subscribe()`, and widgets hold those signals for the lifetime of the
//! window. Restarting a service therefore cannot replace the service value -
//! that would orphan every existing subscriber. Instead the *run* is
//! replaced: [`Lifecycle::begin`] hands the listener a [`RunToken`] tied to a
//! generation counter, and stopping or restarting bumps that counter so the
//! previous listener sees `alive() == false` and bows out.
//!
//! The counter lives in a `watch` channel so a listener parked on a quiet
//! stream does not have to wait for its next event to notice: wrapping the
//! run in [`RunToken::run_until_cancelled`] drops it the moment the
//! generation moves on.
//!
//! A listener does not retry on its own. It returns `Err` and
//! [`RunToken::supervise`] runs it again after a [`Backoff`] delay, so every
//! service recovers from a bus hiccup the same way and shows the same status
//! while it waits.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

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
    generation: Arc<watch::Sender<u64>>,
}

impl Lifecycle {
    pub fn new(mode: ServiceMode) -> Self {
        Self {
            status: Mutable::new(ServiceStatus::Stopped),
            mode: Mutable::new(mode),
            generation: Arc::new(watch::Sender::new(0)),
        }
    }

    fn bump(&self) -> u64 {
        let mut id = 0;
        self.generation.send_modify(|generation| {
            *generation += 1;
            id = *generation;
        });
        id
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
        let id = self.bump();
        let token = RunToken {
            status: self.status.clone(),
            generation: self.generation.subscribe(),
            id,
        };
        token.set(ServiceStatus::Initializing);
        token
    }

    /// Stop the current run and mark the service stopped.
    pub fn stop(&self) {
        self.bump();
        self.status.set_neq(ServiceStatus::Stopped);
    }

    /// Mark a service that has no work to do on this system.
    pub fn set_unavailable(&self) {
        self.bump();
        self.status.set_neq(ServiceStatus::Unavailable);
    }
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new(ServiceMode::default())
    }
}

/// Delay before the next attempt after a failed run: doubles from a second
/// up to a minute, and starts over once a run has stayed up for a while.
#[derive(Debug)]
pub struct Backoff {
    next: Duration,
}

impl Backoff {
    pub const BASE: Duration = Duration::from_secs(1);
    pub const MAX: Duration = Duration::from_secs(60);
    /// A run that lasted this long was healthy, so its failure is a new one.
    pub const HEALTHY: Duration = Duration::from_secs(30);

    pub fn new() -> Self {
        Self { next: Self::BASE }
    }

    /// The delay to wait now, after a run that lasted `ran_for`.
    pub fn delay(&mut self, ran_for: Duration) -> Duration {
        if ran_for >= Self::HEALTHY {
            self.next = Self::BASE;
        }
        let delay = self.next;
        self.next = (self.next * 2).min(Self::MAX);
        delay
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

/// Drive `attempt` on the tokio runtime until it is cancelled or finishes
/// with `Ok`, starting it again after a [`Backoff`] delay each time it fails.
/// The listener only reports its failure by returning it; status, logging
/// and the wait live here. Both are cut short by a stop or restart.
async fn supervise<F, Fut>(token: RunToken, name: &'static str, attempt: F)
where
    F: Fn(RunToken) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let mut backoff = Backoff::new();
    loop {
        let started = tokio::time::Instant::now();
        let err = match token.run_until_cancelled(attempt(token.clone())).await {
            None | Some(Ok(())) => return,
            Some(Err(err)) => err,
        };

        let delay = backoff.delay(started.elapsed());
        tracing::warn!("{name} failed, retrying in {delay:?}: {err:#}");
        token.error(err.to_string());
        if token
            .run_until_cancelled(tokio::time::sleep(delay))
            .await
            .is_none()
        {
            return;
        }
        token.set(ServiceStatus::Initializing);
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
    generation: watch::Receiver<u64>,
    id: u64,
}

impl RunToken {
    /// False once the service was stopped or restarted: exit the listener.
    pub fn alive(&self) -> bool {
        *self.generation.borrow() == self.id
    }

    /// Resolves once the service is stopped or restarted.
    pub async fn cancelled(&self) {
        let id = self.id;
        let mut generation = self.generation.clone();
        // An error means the lifecycle itself is gone, which ends the run too.
        let _ = generation.wait_for(|current| *current != id).await;
    }

    /// Drive `fut` until it completes or this run is superseded.
    ///
    /// On cancellation `fut` is dropped at its current await point, so a
    /// listener parked on a quiet stream releases its thread, proxies and
    /// match rules right away, and none of its pending writes can land.
    pub async fn run_until_cancelled<F: Future>(&self, fut: F) -> Option<F::Output> {
        tokio::select! {
            biased;
            _ = self.cancelled() => None,
            output = fut => Some(output),
        }
    }

    /// Run `attempt` as this run's listener, retrying it with backoff
    /// whenever it returns `Err`. See [`supervise`].
    ///
    /// Call from `ManagedService::start` with the new run's token; `name`
    /// labels the retry in the log.
    pub fn supervise<F, Fut>(self, name: &'static str, attempt: F)
    where
        F: Fn(RunToken) -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        tokio::spawn(supervise(self, name, attempt));
    }

    /// Mutate `data` only while this run is current.
    ///
    /// For listeners that cannot be dropped mid-run (blocking threads), so a
    /// stale one never overwrites what the next run published.
    pub fn write<T>(&self, data: &Mutable<T>, f: impl FnOnce(&mut T)) {
        if self.alive() {
            f(&mut data.lock_mut());
        }
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

    #[tokio::test]
    async fn stop_wakes_a_parked_run() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();

        // A listener parked on a stream that never yields.
        let run = tokio::spawn({
            let token = token.clone();
            async move {
                token
                    .run_until_cancelled(std::future::pending::<()>())
                    .await
            }
        });

        tokio::task::yield_now().await;
        lifecycle.stop();
        assert_eq!(run.await.unwrap(), None);
    }

    #[tokio::test]
    async fn restart_cancels_only_the_previous_run() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let first = lifecycle.begin();
        let second = lifecycle.begin();

        first.cancelled().await;
        assert_eq!(second.run_until_cancelled(async { 7 }).await, Some(7));
    }

    #[test]
    fn stale_writes_are_dropped() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let data = Mutable::new(0);

        let first = lifecycle.begin();
        first.write(&data, |v| *v = 1);
        let second = lifecycle.begin();
        first.write(&data, |v| *v = 2);
        assert_eq!(data.get(), 1);

        second.write(&data, |v| *v = 3);
        assert_eq!(data.get(), 3);
    }

    #[test]
    fn backoff_doubles_to_the_cap_and_resets_after_a_healthy_run() {
        let mut backoff = Backoff::new();
        let instant = Duration::ZERO;

        let delays: Vec<_> = (0..8).map(|_| backoff.delay(instant).as_secs()).collect();
        assert_eq!(delays, [1, 2, 4, 8, 16, 32, 60, 60]);

        assert_eq!(backoff.delay(Backoff::HEALTHY).as_secs(), 1);
        assert_eq!(backoff.delay(instant).as_secs(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_failing_listener_is_retried_until_it_succeeds() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();
        let attempts = Arc::new(AtomicUsize::new(0));
        let started = tokio::time::Instant::now();

        supervise(token, "test", {
            let attempts = attempts.clone();
            move |token| {
                let attempts = attempts.clone();
                async move {
                    match attempts.fetch_add(1, Ordering::SeqCst) {
                        n if n < 3 => anyhow::bail!("attempt {n} failed"),
                        _ => {
                            token.active();
                            Ok(())
                        }
                    }
                }
            }
        })
        .await;

        // Three failures cost 1s + 2s + 4s before the fourth attempt.
        assert_eq!(attempts.load(Ordering::SeqCst), 4);
        assert_eq!(started.elapsed(), Duration::from_secs(7));
        assert_eq!(lifecycle.status(), ServiceStatus::Active);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stop_ends_supervision_instead_of_a_retry() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let token = lifecycle.begin();
        let attempts = Arc::new(AtomicUsize::new(0));

        supervise(token, "test", {
            let attempts = attempts.clone();
            let lifecycle = lifecycle.clone();
            move |_token| {
                let attempts = attempts.clone();
                let lifecycle = lifecycle.clone();
                async move {
                    // The user stops the service while its second attempt fails.
                    if attempts.fetch_add(1, Ordering::SeqCst) == 1 {
                        lifecycle.stop();
                    }
                    anyhow::bail!("down")
                }
            }
        })
        .await;

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(lifecycle.status(), ServiceStatus::Stopped);
    }
}
