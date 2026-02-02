//! Audio service for volume control via WirePlumber/PipeWire.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! audio sink (output) and source (input) volumes using wpctl.

use std::process::Command;
use std::thread;
use std::time::Duration;

use futures_signals::signal::{Mutable, MutableSignalCloned};
use tracing::{debug, error, warn};

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
/// This subscriber monitors audio state via wpctl polling
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct AudioSubscriber {
    data: Mutable<AudioData>,
}

impl AudioSubscriber {
    /// Create a new audio subscriber and start monitoring.
    ///
    /// Polls wpctl every 500ms to detect external volume changes.
    pub fn new() -> Self {
        let initial_data = fetch_audio_data();
        let data = Mutable::new(initial_data);

        // Start the polling listener
        start_listener(data.clone());

        Self { data }
    }

    /// Get a signal that emits when audio state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<AudioData> {
        self.data.signal_cloned()
    }

    /// Get the current audio data snapshot.
    pub fn get(&self) -> AudioData {
        self.data.get_cloned()
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
                } else {
                    self.data.lock_mut().sink_volume = volume.min(100);
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
                } else {
                    self.data.lock_mut().source_volume = volume.min(100);
                }
            }
            AudioCommand::ToggleSinkMute => {
                let result = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                    .output();

                if let Err(e) = result {
                    error!("Failed to toggle sink mute: {}", e);
                } else {
                    let mut data = self.data.lock_mut();
                    data.sink_muted = !data.sink_muted;
                }
            }
            AudioCommand::ToggleSourceMute => {
                let result = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
                    .output();

                if let Err(e) = result {
                    error!("Failed to toggle source mute: {}", e);
                } else {
                    let mut data = self.data.lock_mut();
                    data.source_muted = !data.source_muted;
                }
            }
            AudioCommand::AdjustSinkVolume(delta) => {
                let delta_float = (delta as f32).abs() / 100.0;
                let sign = if delta >= 0 { "+" } else { "-" };
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "-l",
                        "1.0", // Limit to 100%
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{:.2}{}", delta_float, sign),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to adjust sink volume: {}", e);
                } else {
                    let mut data = self.data.lock_mut();
                    data.sink_volume = (data.sink_volume as i16 + delta as i16).clamp(0, 100) as u8;
                }
            }
            AudioCommand::AdjustSourceVolume(delta) => {
                let delta_float = (delta as f32).abs() / 100.0;
                let sign = if delta >= 0 { "+" } else { "-" };
                let result = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "-l",
                        "1.0", // Limit to 100%
                        "@DEFAULT_AUDIO_SOURCE@",
                        &format!("{:.2}{}", delta_float, sign),
                    ])
                    .output();

                if let Err(e) = result {
                    error!("Failed to adjust source volume: {}", e);
                } else {
                    let mut data = self.data.lock_mut();
                    data.source_volume =
                        (data.source_volume as i16 + delta as i16).clamp(0, 100) as u8;
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

/// Start the polling listener thread.
fn start_listener(data: Mutable<AudioData>) {
    thread::spawn(move || {
        loop {
            let new_data = fetch_audio_data();
            let current = data.lock_ref().clone();

            // Only update if changed to avoid unnecessary signal emissions
            if new_data != current {
                debug!(
                    "Audio state changed: sink={}% (muted={}), source={}% (muted={})",
                    new_data.sink_volume,
                    new_data.sink_muted,
                    new_data.source_volume,
                    new_data.source_muted
                );
                *data.lock_mut() = new_data;
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}

/// Fetch current audio data from wpctl.
fn fetch_audio_data() -> AudioData {
    let (sink_volume, sink_muted) = get_volume("@DEFAULT_AUDIO_SINK@");
    let (source_volume, source_muted) = get_volume("@DEFAULT_AUDIO_SOURCE@");

    AudioData {
        sink_volume,
        sink_muted,
        source_volume,
        source_muted,
    }
}

/// Get volume and mute state for a device.
fn get_volume(device: &str) -> (u8, bool) {
    let output = Command::new("wpctl")
        .args(["get-volume", device])
        .output()
        .ok();

    if let Some(output) = output {
        if !output.status.success() {
            warn!("wpctl get-volume failed for {}", device);
            return (0, false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Format: "Volume: 0.50" or "Volume: 0.50 [MUTED]"
        let muted = stdout.contains("[MUTED]");
        let volume = stdout
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<f32>().ok())
            .map(|v| (v * 100.0).round() as u8)
            .unwrap_or(0);

        return (volume, muted);
    }

    (0, false)
}
