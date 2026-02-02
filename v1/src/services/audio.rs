use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use std::ops::Deref;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

/// Audio device data.
#[derive(Debug, Clone, Default)]
pub struct AudioData {
    pub sink_volume: u8,
    pub sink_muted: bool,
    pub source_volume: u8,
    pub source_muted: bool,
}

/// Events from Audio service.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    StateChanged(AudioData),
}

/// Commands for Audio service.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    SetSinkVolume(u8),
    SetSourceVolume(u8),
    ToggleSinkMute,
    ToggleSourceMute,
    AdjustSinkVolume(i8), // +/- percentage
}

/// Audio service for volume control via WirePlumber/PipeWire.
#[derive(Debug, Clone, Default)]
pub struct Audio {
    pub data: AudioData,
}

impl Deref for Audio {
    type Target = AudioData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for Audio {
    type UpdateEvent = AudioEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            AudioEvent::StateChanged(data) => {
                self.data = data;
            }
        }
    }
}

impl Audio {
    /// Create a new GPUI Entity for the Audio service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Audio>>();

        // Spawn a polling thread for audio state
        std::thread::spawn(move || {
            loop {
                let data = fetch_audio_data();
                let _ = tx.send(ServiceEvent::Update(AudioEvent::StateChanged(data)));
                std::thread::sleep(Duration::from_millis(500));
            }
        });

        // Poll the channel for updates
        cx.spawn(async move |this, cx| {
            loop {
                let mut last_event = None;
                while let Ok(event) = rx.try_recv() {
                    last_event = Some(event);
                }

                if let Some(event) = last_event {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            match event {
                                ServiceEvent::Init(audio) => {
                                    this.data = audio.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("Audio service error: {}", e);
                                }
                            }
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        break;
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Audio {
            data: fetch_audio_data(),
        }
    }

    /// Execute an audio command.
    pub fn dispatch(&mut self, command: AudioCommand, cx: &mut Context<Self>) {
        match command {
            AudioCommand::SetSinkVolume(volume) => {
                // wpctl uses floating point: 1.0 = 100%
                let vol_float = volume as f32 / 100.0;
                let _ = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{:.2}", vol_float),
                    ])
                    .spawn();
                self.data.sink_volume = volume;
            }
            AudioCommand::SetSourceVolume(volume) => {
                // wpctl uses floating point: 1.0 = 100%
                let vol_float = volume as f32 / 100.0;
                let _ = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SOURCE@",
                        &format!("{:.2}", vol_float),
                    ])
                    .spawn();
                self.data.source_volume = volume;
            }
            AudioCommand::ToggleSinkMute => {
                let _ = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                    .spawn();
                self.data.sink_muted = !self.data.sink_muted;
            }
            AudioCommand::ToggleSourceMute => {
                let _ = Command::new("wpctl")
                    .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
                    .spawn();
                self.data.source_muted = !self.data.source_muted;
            }
            AudioCommand::AdjustSinkVolume(delta) => {
                // wpctl uses floating point for step: 0.05 = 5%
                let delta_float = (delta as f32).abs() / 100.0;
                let sign = if delta >= 0 { "+" } else { "-" };
                let _ = Command::new("wpctl")
                    .args([
                        "set-volume",
                        "-l",
                        "1.0", // Limit to 100%
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{:.2}{}", delta_float, sign),
                    ])
                    .spawn();
                self.data.sink_volume =
                    (self.data.sink_volume as i16 + delta as i16).clamp(0, 100) as u8;
            }
        }
        cx.notify();
    }
}

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

fn get_volume(device: &str) -> (u8, bool) {
    let output = Command::new("wpctl")
        .args(["get-volume", device])
        .output()
        .ok();

    if let Some(output) = output {
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
