//! Per-service command queue.
//!
//! UI code sends commands with [`JobQueue::send`] (fire and forget) or
//! [`JobQueue::request`] (await the outcome) and never blocks or spawns.
//! Each run of a service drains its own queue on one worker, so commands
//! execute one at a time in the order they were sent - two quick toggles
//! cannot both read the same "before" state - and a stop or restart retires
//! the worker together with the listener.

use std::collections::VecDeque;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Result, anyhow};
use futures_signals::signal::{Mutable, Signal};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

use crate::lifecycle::RunToken;

struct Job<C> {
    command: C,
    reply: Option<oneshot::Sender<Result<()>>>,
    /// Keeps the command listed in [`JobQueue::in_flight`] until the job is
    /// dropped - run, superseded, or discarded with its run.
    _ticket: Ticket<C>,
}

type InFlight<C> = Mutable<Vec<(u64, C)>>;

struct Ticket<C> {
    id: u64,
    in_flight: InFlight<C>,
}

impl<C> Drop for Ticket<C> {
    fn drop(&mut self) {
        let mut in_flight = self.in_flight.lock_mut();
        if let Some(index) = in_flight.iter().position(|(id, _)| *id == self.id) {
            in_flight.remove(index);
        }
    }
}

type Waker = Box<dyn Fn() + Send + Sync>;

struct Inlet<C> {
    tx: mpsc::UnboundedSender<Job<C>>,
    /// Nudges a worker that is not a tokio task (a native mainloop thread)
    /// to look at its queue.
    wake: Option<Waker>,
}

/// Commands queued for the current run of one service.
pub struct JobQueue<C> {
    name: &'static str,
    inlet: Arc<Mutex<Option<Inlet<C>>>>,
    in_flight: InFlight<C>,
    next_ticket: Arc<AtomicU64>,
    /// True when `newer` makes the queued `older` pointless (two absolute
    /// volume sets during a slider drag): `older` is skipped.
    supersedes: Option<fn(&C, &C) -> bool>,
}

impl<C> Clone for JobQueue<C> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            inlet: self.inlet.clone(),
            in_flight: self.in_flight.clone(),
            next_ticket: self.next_ticket.clone(),
            supersedes: self.supersedes,
        }
    }
}

impl<C> Debug for JobQueue<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobQueue")
            .field("name", &self.name)
            .finish()
    }
}

impl<C> Default for JobQueue<C> {
    /// A queue named after its command type, e.g. `NetworkCommand`.
    fn default() -> Self {
        let name = std::any::type_name::<C>();
        Self {
            name: name.rsplit("::").next().unwrap_or(name),
            inlet: Arc::default(),
            in_flight: Mutable::default(),
            next_ticket: Arc::default(),
            supersedes: None,
        }
    }
}

impl<C: Clone + Debug + Send + Sync + 'static> JobQueue<C> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            inlet: Arc::default(),
            in_flight: Mutable::default(),
            next_ticket: Arc::default(),
            supersedes: None,
        }
    }

    /// Skip a queued command when a later one supersedes it.
    pub fn coalesce(mut self, supersedes: fn(&C, &C) -> bool) -> Self {
        self.supersedes = Some(supersedes);
        self
    }

    /// Commands queued or running, oldest first. Lets the UI show progress
    /// for, say, the one device being connected.
    pub fn in_flight(&self) -> Vec<C> {
        self.in_flight
            .lock_ref()
            .iter()
            .map(|(_, c)| c.clone())
            .collect()
    }

    /// Signal form of [`Self::in_flight`].
    pub fn in_flight_signal(&self) -> impl Signal<Item = Vec<C>> + Unpin + use<C> {
        self.in_flight
            .signal_ref(|in_flight| in_flight.iter().map(|(_, c)| c.clone()).collect())
    }

    fn job(&self, command: C, reply: Option<oneshot::Sender<Result<()>>>) -> Job<C> {
        let id = self.next_ticket.fetch_add(1, Ordering::Relaxed);
        self.in_flight.lock_mut().push((id, command.clone()));
        Job {
            command,
            reply,
            _ticket: Ticket {
                id,
                in_flight: self.in_flight.clone(),
            },
        }
    }

    /// Queue `command`. Failures are logged by the worker.
    pub fn send(&self, command: C) {
        self.push(self.job(command, None));
    }

    /// Queue `command` and wait for its outcome.
    pub fn request(&self, command: C) -> impl Future<Output = Result<()>> + use<C> {
        let (reply, outcome) = oneshot::channel();
        self.push(self.job(command, Some(reply)));
        async move {
            outcome
                .await
                .unwrap_or_else(|_| Err(anyhow!("service stopped before running the command")))
        }
    }

    fn push(&self, job: Job<C>) {
        let inlet = self.inlet.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(inlet) = inlet.as_ref() else {
            warn!(service = self.name, command = ?job.command, "service not running, command dropped");
            if let Some(reply) = job.reply {
                let _ = reply.send(Err(anyhow!("{} is not running", self.name)));
            }
            return;
        };
        if let Err(mpsc::error::SendError(job)) = inlet.tx.send(job) {
            warn!(service = self.name, command = ?job.command, "service worker gone, command dropped");
            return;
        }
        if let Some(wake) = &inlet.wake {
            wake();
        }
    }

    /// Open the queue for a new run, closing the previous run's receiver.
    fn open(&self, wake: Option<Waker>) -> mpsc::UnboundedReceiver<Job<C>> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.inlet.lock().unwrap_or_else(PoisonError::into_inner) = Some(Inlet { tx, wake });
        rx
    }

    /// Run `handler` for every queued command until `token` is superseded.
    ///
    /// Call from `ManagedService::start` with the new run's token.
    pub fn run<F, Fut>(&self, token: RunToken, mut handler: F)
    where
        F: FnMut(C) -> Fut + Send + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        let mut rx = self.open(None);
        let name = self.name;
        let supersedes = self.supersedes;

        tokio::spawn(async move {
            let mut pending = VecDeque::new();
            loop {
                if pending.is_empty() {
                    match token.run_until_cancelled(rx.recv()).await {
                        Some(Some(job)) => pending.push_back(job),
                        _ => break,
                    }
                }
                while let Ok(job) = rx.try_recv() {
                    pending.push_back(job);
                }
                let Some(job) = next_job(&mut pending, supersedes) else {
                    continue;
                };

                debug!(service = name, command = ?job.command, "running command");
                let label = format!("{:?}", job.command);
                let Some(outcome) = token.run_until_cancelled(handler(job.command)).await else {
                    break;
                };
                if let Err(e) = &outcome {
                    warn!(service = name, command = %label, "command failed: {e:#}");
                }
                if let Some(reply) = job.reply {
                    let _ = reply.send(outcome);
                }
            }
        });
    }

    /// Hand the queue to a worker that is not a tokio task.
    ///
    /// `wake` is called after every send so a thread blocked in a native
    /// event loop can come back and [`JobReceiver::drain`] it.
    pub fn attach(&self, wake: impl Fn() + Send + Sync + 'static) -> JobReceiver<C> {
        JobReceiver {
            name: self.name,
            rx: self.open(Some(Box::new(wake))),
            supersedes: self.supersedes,
        }
    }
}

/// Pop the next job to run, skipping (and acknowledging) any that a later
/// queued job supersedes.
fn next_job<C>(
    pending: &mut VecDeque<Job<C>>,
    supersedes: Option<fn(&C, &C) -> bool>,
) -> Option<Job<C>> {
    let job = pending.pop_front()?;
    if let Some(supersedes) = supersedes
        && pending
            .iter()
            .any(|later| supersedes(&job.command, &later.command))
    {
        if let Some(reply) = job.reply {
            let _ = reply.send(Ok(()));
        }
        return None;
    }
    Some(job)
}

/// The receiving end of a [`JobQueue`] owned by a blocking worker.
pub struct JobReceiver<C> {
    name: &'static str,
    rx: mpsc::UnboundedReceiver<Job<C>>,
    supersedes: Option<fn(&C, &C) -> bool>,
}

impl<C: Debug> JobReceiver<C> {
    /// Run `handler` on every command queued so far, in order.
    pub fn drain(&mut self, mut handler: impl FnMut(C) -> Result<()>) {
        let mut pending = VecDeque::new();
        while let Ok(job) = self.rx.try_recv() {
            pending.push_back(job);
        }
        while !pending.is_empty() {
            let Some(job) = next_job(&mut pending, self.supersedes) else {
                continue;
            };
            let label = format!("{:?}", job.command);
            let outcome = handler(job.command);
            if let Err(e) = &outcome {
                warn!(service = self.name, command = %label, "command failed: {e:#}");
            }
            if let Some(reply) = job.reply {
                let _ = reply.send(outcome);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{Lifecycle, ServiceMode};
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq)]
    enum Cmd {
        Set(u32),
        Add(u32),
        Fail,
    }

    fn handler(
        log: Arc<Mutex<Vec<Cmd>>>,
    ) -> impl FnMut(Cmd) -> futures_util::future::Ready<Result<()>> {
        move |cmd| {
            let failed = cmd == Cmd::Fail;
            log.lock().unwrap().push(cmd);
            futures_util::future::ready(if failed { Err(anyhow!("boom")) } else { Ok(()) })
        }
    }

    #[tokio::test]
    async fn commands_run_in_order_and_report_outcomes() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let queue = JobQueue::new("test");
        let log = Arc::new(Mutex::new(Vec::new()));
        queue.run(lifecycle.begin(), handler(log.clone()));

        queue.send(Cmd::Add(1));
        queue.send(Cmd::Add(2));
        assert!(queue.request(Cmd::Fail).await.is_err());
        assert!(queue.request(Cmd::Add(3)).await.is_ok());

        assert_eq!(
            *log.lock().unwrap(),
            [Cmd::Add(1), Cmd::Add(2), Cmd::Fail, Cmd::Add(3)]
        );
    }

    #[tokio::test]
    async fn superseded_commands_are_skipped() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let queue = JobQueue::new("test")
            .coalesce(|older, newer| matches!((older, newer), (Cmd::Set(_), Cmd::Set(_))));
        let log = Arc::new(Mutex::new(Vec::new()));

        // Queue everything before the worker exists so it sees one batch.
        let mut rx = queue.open(None);
        for cmd in [Cmd::Set(1), Cmd::Set(2), Cmd::Add(5), Cmd::Set(3)] {
            queue.send(cmd);
        }
        let mut pending = VecDeque::new();
        while let Ok(job) = rx.try_recv() {
            pending.push_back(job);
        }
        let mut ran = Vec::new();
        while !pending.is_empty() {
            if let Some(job) = next_job(&mut pending, queue.supersedes) {
                ran.push(job.command);
            }
        }
        assert_eq!(ran, [Cmd::Add(5), Cmd::Set(3)]);

        // And end to end, through a worker.
        queue.run(lifecycle.begin(), handler(log.clone()));
        queue.send(Cmd::Set(7));
        queue.request(Cmd::Set(8)).await.unwrap();
        assert_eq!(log.lock().unwrap().last(), Some(&Cmd::Set(8)));
    }

    #[tokio::test]
    async fn stopping_retires_the_worker() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let queue = JobQueue::new("test");

        // Not started yet: the command is refused, not lost silently.
        assert!(queue.request(Cmd::Add(1)).await.is_err());

        // A command that never finishes is dropped by the stop.
        queue.run(lifecycle.begin(), |_| std::future::pending::<Result<()>>());
        let parked = queue.request(Cmd::Add(1));
        tokio::time::sleep(Duration::from_millis(10)).await;
        lifecycle.stop();
        assert!(parked.await.is_err());
    }

    #[tokio::test]
    async fn in_flight_lists_commands_until_they_finish() {
        let lifecycle = Lifecycle::new(ServiceMode::Eager);
        let queue = JobQueue::new("test");
        let (release, released) = oneshot::channel::<()>();
        let released = Arc::new(Mutex::new(Some(released)));
        queue.run(lifecycle.begin(), move |_| {
            let released = released.lock().unwrap().take();
            async move {
                if let Some(released) = released {
                    let _ = released.await;
                }
                Ok(())
            }
        });

        let first = queue.request(Cmd::Add(1));
        queue.send(Cmd::Add(2));
        assert_eq!(queue.in_flight(), [Cmd::Add(1), Cmd::Add(2)]);

        release.send(()).unwrap();
        first.await.unwrap();
        queue.request(Cmd::Add(3)).await.unwrap();
        assert!(queue.in_flight().is_empty());

        // A stop discards what was queued, and the list with it.
        queue.send(Cmd::Add(4));
        lifecycle.stop();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(queue.in_flight().is_empty());
    }

    #[test]
    fn attached_receivers_are_woken_and_drained() {
        let queue = JobQueue::new("test");
        let wakes = Arc::new(Mutex::new(0));
        let mut rx = queue.attach({
            let wakes = wakes.clone();
            move || *wakes.lock().unwrap() += 1
        });

        queue.send(Cmd::Add(1));
        queue.send(Cmd::Add(2));
        assert_eq!(*wakes.lock().unwrap(), 2);

        let mut ran = Vec::new();
        rx.drain(|cmd| {
            ran.push(cmd);
            Ok(())
        });
        assert_eq!(ran, [Cmd::Add(1), Cmd::Add(2)]);
    }
}
