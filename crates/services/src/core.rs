//! Shared reactive state, lifecycle and command queue owned by a service.

use std::fmt::Debug;
use std::future::Future;

use futures_signals::signal::{Mutable, MutableSignalCloned, Signal};

use crate::{JobQueue, Lifecycle};

#[derive(Debug, Clone)]
pub struct ServiceCore<D, C> {
    pub(crate) data: Mutable<D>,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) jobs: JobQueue<C>,
}

impl<D: Default, C> Default for ServiceCore<D, C> {
    fn default() -> Self {
        Self {
            data: Mutable::default(),
            lifecycle: Lifecycle::default(),
            jobs: JobQueue::default(),
        }
    }
}

impl<D: Default, C> ServiceCore<D, C> {
    pub fn with_jobs(jobs: JobQueue<C>) -> Self {
        Self {
            jobs,
            ..Self::default()
        }
    }
}

impl<D: Clone, C> ServiceCore<D, C> {
    pub fn subscribe(&self) -> MutableSignalCloned<D> {
        self.data.signal_cloned()
    }

    pub fn get(&self) -> D {
        self.data.get_cloned()
    }
}

impl<D, C: Clone + Debug + Send + Sync + 'static> ServiceCore<D, C> {
    pub fn dispatch(&self, command: C) {
        self.jobs.send(command);
    }

    pub fn request(&self, command: C) -> impl Future<Output = anyhow::Result<()>> + use<D, C> {
        self.jobs.request(command)
    }

    pub fn pending(&self) -> Vec<C> {
        self.jobs.in_flight()
    }

    pub fn pending_signal(&self) -> impl Signal<Item = Vec<C>> + Unpin + use<D, C> {
        self.jobs.in_flight_signal()
    }
}
