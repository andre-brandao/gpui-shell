pub mod compositor;
pub mod network;

use compositor::Compositor;
use gpui::{App, AppContext, Entity};
use network::Network;

/// Event wrapper for service state changes.
#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    /// Initial state when service starts.
    Init(S),
    /// State update event.
    Update(S::UpdateEvent),
    /// Error occurred in the service.
    Error(S::Error),
}

/// A service that can receive commands and mutate state.
pub trait Service: ReadOnlyService {
    type Command: Send + 'static;

    /// Execute a command asynchronously.
    fn command(
        &mut self,
        command: Self::Command,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}

/// A read-only service that receives updates from a background task.
pub trait ReadOnlyService: Sized + Clone + Send + 'static {
    type UpdateEvent: Clone + Send + 'static;
    type Error: Clone + Send + 'static;

    /// Apply an update event to the service state.
    fn update(&mut self, event: Self::UpdateEvent);
}

/// Container holding all service entities.
/// Pass this to components that need access to multiple services.
#[derive(Clone)]
pub struct Services {
    pub compositor: Entity<Compositor>,
    pub network: Entity<Network>,
}

impl Services {
    /// Create all services. Call this once at app startup.
    pub fn new(cx: &mut App) -> Self {
        Services {
            compositor: cx.new(Compositor::new),
            network: cx.new(Network::new),
        }
    }
}
