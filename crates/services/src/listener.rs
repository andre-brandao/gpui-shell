//! Shared scaffolding for service listener loops.
//!
//! Every service needs the same lifecycle around its listener: run it, report
//! failures through [`ServiceStatus`], and restart with backoff instead of
//! dying silently. These helpers own that loop so services only implement the
//! actual listening.
//!
//! The `run` closure should (re)connect, bring `data` up to date, set the
//! status to [`ServiceStatus::Active`], and then listen until the connection
//! fails. Returning `Ok` (stream exhausted) restarts just like an error, but
//! without flipping the status to `Error`.

use std::time::{Duration, Instant};

use futures_signals::signal::Mutable;

use crate::ServiceStatus;

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// A listener that ran at least this long is considered to have connected
/// successfully, resetting the restart backoff.
const HEALTHY_RUN: Duration = Duration::from_secs(60);

/// Spawn an async listener on the shared Tokio runtime, restarting it with
/// exponential backoff whenever it ends.
pub(crate) fn spawn_listener<F, Fut>(name: &'static str, status: Mutable<ServiceStatus>, run: F)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send,
{
    tokio::spawn(async move {
        let mut backoff = INITIAL_BACKOFF;
        loop {
            let started = Instant::now();
            report_run_end(name, &status, run().await);
            backoff = next_backoff(backoff, started.elapsed());
            tokio::time::sleep(backoff).await;
        }
    });
}

/// Spawn a blocking listener on a dedicated OS thread, restarting it with
/// exponential backoff whenever it ends.
///
/// For listeners that cannot run as a task on the shared runtime: blocking
/// mainloops (libpulse, pipewire), blocking sockets, or `!Send` state.
pub(crate) fn spawn_blocking_listener(
    name: &'static str,
    status: Mutable<ServiceStatus>,
    run: impl Fn() -> anyhow::Result<()> + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut backoff = INITIAL_BACKOFF;
        loop {
            let started = Instant::now();
            report_run_end(name, &status, run());
            backoff = next_backoff(backoff, started.elapsed());
            std::thread::sleep(backoff);
        }
    });
}

fn report_run_end(name: &str, status: &Mutable<ServiceStatus>, result: anyhow::Result<()>) {
    match result {
        Ok(()) => tracing::warn!("{name} listener ended; restarting"),
        Err(e) => {
            tracing::error!("{name} listener error: {e:#}");
            *status.lock_mut() = ServiceStatus::Error(Some(e.to_string()));
        }
    }
}

fn next_backoff(current: Duration, ran_for: Duration) -> Duration {
    if ran_for >= HEALTHY_RUN {
        INITIAL_BACKOFF
    } else {
        (current * 2).min(MAX_BACKOFF)
    }
}
