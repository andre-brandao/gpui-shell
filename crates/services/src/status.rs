//! Service status for health monitoring.
//!
//! This module provides a standard status enum that services can use
//! to report their health and availability to the UI layer.

/// Standard service status for all services with background tasks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServiceStatus {
    /// Service is running and operational.
    Active,
    /// Service is starting up.
    #[default]
    Initializing,
    /// Service stopped due to error.
    Error(Option<String>),
    /// Service intentionally disabled or unavailable.
    Unavailable,
}

impl ServiceStatus {
    /// Check if the service is operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, ServiceStatus::Active)
    }

    /// Get a Nerd Font icon representing the status.
    pub fn icon(&self) -> &'static str {
        match self {
            ServiceStatus::Active => "󰄬",       // Check mark
            ServiceStatus::Initializing => "󰥔", // Loading spinner
            ServiceStatus::Error(_) => "󰅚",     // Error X
            ServiceStatus::Unavailable => "󰪎",  // Disabled
        }
    }

    /// Get a human-readable label for the status.
    pub fn label(&self) -> &'static str {
        match self {
            ServiceStatus::Active => "Active",
            ServiceStatus::Initializing => "Starting",
            ServiceStatus::Error(_) => "Error",
            ServiceStatus::Unavailable => "Unavailable",
        }
    }

    /// Get the error message if this is an error status.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ServiceStatus::Error(msg) => msg.as_deref(),
            _ => None,
        }
    }
}
