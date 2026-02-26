//! Audio service for volume control via PulseAudio/PipeWire.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! audio sink (output) and source (input) volumes using libpulse for monitoring
//! and wpctl for commands.

use std::cell::{Cell, RefCell};
use std::process::Command;
use std::rc::Rc;
use std::thread;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use libpulse_binding::{
    context::{self, Context, FlagSet, subscribe::InterestMaskSet},
    mainloop::standard::{IterateResult, Mainloop},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::Volume,
};
use tracing::{debug, error};

use crate::ServiceStatus;

/// Audio device data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioData {
    /// Sink (output) volume as percentage (0-100).
    pub sink_volume: u8,
    /// Whether the sink is muted.
    pub sink_muted: bool,
    /// Source (input) volume as percentage (0-100).
    pub source_volume: u8,
    /// Whether the source is muted.
    pub source_muted: bool,
}

impl AudioData {
    /// Get an icon based on sink volume and mute state.
    pub fn sink_icon(&self) -> &'static str {
        if self.sink_muted {
            "󰝟"
        } else {
            match self.sink_volume {
                0 => "󰕿",
                1..=50 => "󰖀",
                _ => "󰕾",
            }
        }
    }

    /// Get an icon based on source volume and mute state.
    pub fn source_icon(&self) -> &'static str {
        if self.source_muted { "󰍭" } else { "󰍬" }
    }
}

/// Commands for controlling audio.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Set sink (output) volume as percentage (0-100).
    SetSinkVolume(u8),
    /// Set source (input) volume as percentage (0-100).
    SetSourceVolume(u8),
    /// Toggle sink mute state.
    ToggleSinkMute,
    /// Toggle source mute state.
    ToggleSourceMute,
    /// Adjust sink volume by delta percentage (+/-).
    AdjustSinkVolume(i8),
    /// Adjust source volume by delta percentage (+/-).
    AdjustSourceVolume(i8),
}

/// Event-driven audio subscriber.
///
/// This subscriber monitors audio state via PulseAudio's subscribe API
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct AudioSubscriber {
    data: Mutable<AudioData>,
    status: Mutable<ServiceStatus>,
}

impl AudioSubscriber {
    /// Create a new audio subscriber and start monitoring.
    pub fn new() -> Self {
        let data = Mutable::new(AudioData::default());
        let status = Mutable::new(ServiceStatus::Initializing);
        start_listener(data.clone(), status.clone());
        Self { data, status }
    }

    /// Get a signal that emits when audio state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<AudioData> {
        self.data.signal_cloned()
    }

    /// Get the current audio data snapshot.
    pub fn get(&self) -> AudioData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }

    /// Execute an audio command.
    pub fn dispatch(&self, command: AudioCommand) {
        match command {
            AudioCommand::SetSinkVolume(volume) => {
                let vol_float = volume.min(100) as f32 / 100.0;
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{:.2}", vol_float),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to set sink volume: {}", e);
                }
            }
            AudioCommand::SetSourceVolume(volume) => {
                let vol_float = volume.min(100) as f32 / 100.0;
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SOURCE@",
                        &format!("{:.2}", vol_float),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to set source volume: {}", e);
                }
            }
            AudioCommand::ToggleSinkMute => {
                let result = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                    .output();

                if let Err(e) = result {
                    error!("Failed to toggle sink mute: {}", e);
                }
            }
            AudioCommand::ToggleSourceMute => {
                let result = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
                    .output();

                if let Err(e) = result {
                    error!("Failed to toggle source mute: {}", e);
                }
            }
            AudioCommand::AdjustSinkVolume(delta) => {
                let delta_float = (delta as f32).abs() / 100.0;
                let sign = if delta >= 0 { "+" } else { "-" };
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "-l",
                        "1.0",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{:.2}{}", delta_float, sign),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to adjust sink volume: {}", e);
                }
            }
            AudioCommand::AdjustSourceVolume(delta) => {
                let delta_float = (delta as f32).abs() / 100.0;
                let sign = if delta >= 0 { "+" } else { "-" };
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "-l",
                        "1.0",
                        "@DEFAULT_AUDIO_SOURCE@",
                        &format!("{:.2}{}", delta_float, sign),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to adjust source volume: {}", e);
                }
            }
        }
    }
}

impl Default for AudioSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert PulseAudio volume to percentage (0-100).
fn volume_to_percent(volume: Volume) -> u8 {
    let ratio = volume.0 as f64 / Volume::NORMAL.0 as f64;
    (ratio * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Start the PulseAudio event listener thread.
fn start_listener(data: Mutable<AudioData>, status: Mutable<ServiceStatus>) {
    thread::spawn(move || {
        let mut proplist = Proplist::new().expect("Failed to create PulseAudio proplist");
        let _ = proplist.set_str(APPLICATION_NAME, "gpuishell");

        let mut mainloop = Mainloop::new().expect("Failed to create PulseAudio mainloop");

        let mut context = Context::new_with_proplist(&mainloop, "gpuishell", &proplist)
            .expect("Failed to create PulseAudio context");

        context
            .connect(None, FlagSet::NOFLAGS, None)
            .expect("Failed to connect to PulseAudio");

        // Wait for context to be ready
        loop {
            match mainloop.iterate(true) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio mainloop error during connect, stopping listener");
                    *status.lock_mut() = ServiceStatus::Error(None);
                    return;
                }
                IterateResult::Success(_) => match context.get_state() {
                    context::State::Ready => break,
                    context::State::Failed | context::State::Terminated => {
                        error!("PulseAudio context failed or terminated, stopping listener");
                        *status.lock_mut() = ServiceStatus::Error(None);
                        return;
                    }
                    _ => {}
                },
            }
        }

        debug!("PulseAudio connection established");
        *status.lock_mut() = ServiceStatus::Active;

        // Shared state for tracking pending queries and results
        let pending_queries = Rc::new(Cell::new(0u32));
        let local_data: Rc<RefCell<AudioData>> = Rc::new(RefCell::new(AudioData::default()));

        // Query functions that track pending state
        let query_sink = {
            let introspector = context.introspect();
            let local_data = local_data.clone();
            let pending = pending_queries.clone();
            move || {
                pending.set(pending.get() + 1);
                let local_data = local_data.clone();
                let pending = pending.clone();
                introspector.get_sink_info_by_name("@DEFAULT_SINK@", move |result| match result {
                    libpulse_binding::callbacks::ListResult::Item(sink) => {
                        let volume = volume_to_percent(sink.volume.avg());
                        let muted = sink.mute;
                        let mut data = local_data.borrow_mut();
                        data.sink_volume = volume;
                        data.sink_muted = muted;
                    }
                    libpulse_binding::callbacks::ListResult::End
                    | libpulse_binding::callbacks::ListResult::Error => {
                        pending.set(pending.get().saturating_sub(1));
                    }
                });
            }
        };

        let query_source = {
            let introspector = context.introspect();
            let local_data = local_data.clone();
            let pending = pending_queries.clone();
            move || {
                pending.set(pending.get() + 1);
                let local_data = local_data.clone();
                let pending = pending.clone();
                introspector.get_source_info_by_name("@DEFAULT_SOURCE@", move |result| {
                    match result {
                        libpulse_binding::callbacks::ListResult::Item(source) => {
                            // Skip monitor sources (they mirror sinks)
                            if source.monitor_of_sink.is_none() {
                                let volume = volume_to_percent(source.volume.avg());
                                let muted = source.mute;
                                let mut data = local_data.borrow_mut();
                                data.source_volume = volume;
                                data.source_muted = muted;
                            }
                        }
                        libpulse_binding::callbacks::ListResult::End
                        | libpulse_binding::callbacks::ListResult::Error => {
                            pending.set(pending.get().saturating_sub(1));
                        }
                    }
                });
            }
        };

        // Fetch initial audio data
        query_sink();
        query_source();

        // Process initial queries with non-blocking iterations
        while pending_queries.get() > 0 {
            match mainloop.iterate(false) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio mainloop error during initial query, stopping listener");
                    *status.lock_mut() = ServiceStatus::Error(None);
                    return;
                }
                IterateResult::Success(_) => {}
            }
        }

        // Update shared state with initial data
        {
            let current = local_data.borrow().clone();
            debug!(
                "Initial audio state: sink={}% (muted={}), source={}% (muted={})",
                current.sink_volume,
                current.sink_muted,
                current.source_volume,
                current.source_muted
            );
            *data.lock_mut() = current;
        }

        // Flag to indicate we need to re-query
        let needs_refresh = Rc::new(Cell::new(false));

        // Subscribe to sink and source changes
        context.subscribe(
            InterestMaskSet::SINK
                .union(InterestMaskSet::SOURCE)
                .union(InterestMaskSet::SERVER),
            |success| {
                if !success {
                    error!("PulseAudio subscription failed");
                }
            },
        );

        // Set callback for subscription events - just mark that we need to refresh
        let needs_refresh_cb = needs_refresh.clone();
        context.set_subscribe_callback(Some(Box::new(move |_facility, _operation, _idx| {
            needs_refresh_cb.set(true);
        })));

        // Main event loop
        loop {
            // Use non-blocking iteration when queries are pending,
            // otherwise block waiting for events
            let has_pending = pending_queries.get() > 0 || needs_refresh.get();
            match mainloop.iterate(!has_pending) {
                IterateResult::Quit(_) | IterateResult::Err(_) => {
                    error!("PulseAudio mainloop error in event loop, stopping listener");
                    *status.lock_mut() = ServiceStatus::Error(None);
                    return;
                }
                IterateResult::Success(_) => {
                    // Check if we need to refresh due to a subscription event
                    if needs_refresh.get() {
                        needs_refresh.set(false);

                        // Fire off queries
                        query_sink();
                        query_source();
                    }

                    // If all pending queries completed, check for changes
                    if pending_queries.get() == 0 {
                        let local = local_data.borrow().clone();
                        let current = data.lock_ref().clone();
                        if local != current {
                            debug!(
                                "Audio state changed: sink={}% (muted={}), source={}% (muted={})",
                                local.sink_volume,
                                local.sink_muted,
                                local.source_volume,
                                local.source_muted
                            );
                            *data.lock_mut() = local;
                        }
                    }
                }
            }
        }
    });
}
