//! Shell service for single-instance IPC via D-Bus.
//!
//! This module provides a service that ensures only one instance of gpuishell
//! runs at a time. When a second instance is launched, it sends a message to
//! the running instance via D-Bus and exits.
//!
//! The service exposes a signal-based interface for reacting to launcher open
//! requests from other processes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use futures_signals::signal::{Mutable, MutableSign