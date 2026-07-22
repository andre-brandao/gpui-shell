//! Shared D-Bus connections.
//!
//! All services share one system-bus and one session-bus connection. The
//! connections are built with zbus's internal executor disabled and are driven
//! by a task on the shared tokio runtime instead — zbus otherwise spawns one
//! dedicated OS thread per connection to tick its executor.

use tokio::sync::OnceCell;
use zbus::{Connection, connection::Builder};

static SYSTEM: OnceCell<Connection> = OnceCell::const_new();
static SESSION: OnceCell<Connection> = OnceCell::const_new();

/// Get the shared system-bus connection, creating it on first use.
pub async fn system() -> zbus::Result<Connection> {
    SYSTEM
        .get_or_try_init(|| build(Builder::system()))
        .await
        .cloned()
}

/// Get the shared session-bus connection, creating it on first use.
pub async fn session() -> zbus::Result<Connection> {
    SESSION
        .get_or_try_init(|| build(Builder::session()))
        .await
        .cloned()
}

async fn build(builder: zbus::Result<Builder<'static>>) -> zbus::Result<Connection> {
    let conn = builder?.internal_executor(false).build().await?;
    let executor = conn.executor().clone();
    tokio::spawn(async move {
        loop {
            executor.tick().await;
        }
    });
    Ok(conn)
}
