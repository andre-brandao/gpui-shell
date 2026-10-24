//! Audio service for volume control via PulseAudio/PipeWire.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! audio sink (output) and source (input) volumes. Both monitoring and
//! commands go through libpulse on one mainloop thread.

use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use anyhow::{Context as _, anyhow, bail};
use futures_signals::signal::{Mutable, MutableSignalCloned};
use libpulse_binding::{
    callbacks::ListResult,
    context::{self, Context, FlagSet, introspect::Introspector, subscribe::InterestMaskSet},
    mainloop::api::Mainloop as _,
    mainloop::events::io::FlagSet as IoEventFlagSet,
    mainloop::standard::{IterateResult, Mainloop},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::{ChannelVolumes, Volume},
};
use tracing::{debug, error};

use crate::jobs::{JobQueue, JobReceiver};
use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

const DEFAULT_SINK: &str = "@DEFAULT_SINK@";
const DEFAULT_SOURCE: &str = "@DEFAULT_SOURCE@";

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
    lifecycle: Lifecycle,
    jobs: JobQueue<AudioCommand>,
}

impl Default for AudioSubscriber {
    fn default() -> Self {
        use AudioCommand::{SetSinkVolume, SetSourceVolume};
        Self {
            data: Mutable::default(),
            lifecycle: Lifecycle::default(),
            // A slider drag only needs its last value per device.
            jobs: JobQueue::new("Audio").coalesce(|older, newer| {
                matches!(
                    (older, newer),
                    (SetSinkVolume(_), SetSinkVolume(_)) | (SetSourceVolume(_), SetSourceVolume(_))
                )
            }),
        }
    }
}

impl AudioSubscriber {
    /// Create a new audio subscriber. Monitoring starts with
    /// [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a signal that emits when audio state changes.
    pub fn subscribe(&self) -> MutableSignalCloned<AudioData> {
        self.data.signal_cloned()
    }

    /// Get the current audio data snapshot.
    pub fn get(&self) -> AudioData {
        self.data.get_cloned()
    }

    /// Queue an audio command. It runs on the PulseAudio thread; failures
    /// are logged.
    pub fn dispatch(&self, command: AudioCommand) {
        self.jobs.send(command);
    }
}

impl ManagedService for AudioSubscriber {
    fn name(&self) -> &'static str {
        "Audio"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        size_of::<AudioData>()
    }

    fn start(&self) {
        let token = self.lifecycle.begin();

        // The mainloop thread sleeps in `iterate(true)` until PulseAudio has
        // something for it. Writing a byte to this pair wakes it for a queued
        // command or a stop.
        let (wake_rx, wake_tx) = match UnixStream::pair().and_then(|(rx, tx)| {
            rx.set_nonblocking(true)?;
            tx.set_nonblocking(true)?;
            Ok((rx, Arc::new(tx)))
        }) {
            Ok(pair) => pair,
            Err(e) => {
                error!("Failed to create the audio wake-up socket: {}", e);
                token.error(e.to_string());
                return;
            }
        };

        let jobs = self.jobs.attach({
            let wake_tx = wake_tx.clone();
            move || wake(&wake_tx)
        });

        tokio::spawn({
            let token = token.clone();
            async move {
                token.cancelled().await;
                wake(&wake_tx);
            }
        });

        let data = self.data.clone();
        thread::spawn(move || {
            if let Err(e) = run(&data, &token, jobs, wake_rx) {
                error!("Audio listener error: {e:#}");
                token.error(format!("{e:#}"));
            }
        });
    }
}

/// Nudge the mainloop thread. A full socket already has a wake-up pending.
fn wake(tx: &UnixStream) {
    let _ = (&*tx).write(&[1]);
}

/// Convert PulseAudio volume to percentage (0-100).
fn volume_to_percent(volume: Volume) -> u8 {
    let ratio = volume.0 as f64 / Volume::NORMAL.0 as f64;
    (ratio * 100.0).round().clamp(0.0, 100.0) as u8
}

/// Convert a percentage (0-100) to a PulseAudio volume.
fn percent_to_volume(percent: u8) -> Volume {
    Volume((Volume::NORMAL.0 as f64 * percent.min(100) as f64 / 100.0).round() as u32)
}

/// What the mainloop thread knows about the default devices.
#[derive(Default)]
struct Local {
    data: AudioData,
    /// Per-channel volumes, kept so a volume change preserves balance.
    sink: Option<ChannelVolumes>,
    source: Option<ChannelVolumes>,
}

/// One run of the PulseAudio thread: connect, snapshot, then follow events
/// and run queued commands until the token is superseded.
fn run(
    data: &Mutable<AudioData>,
    token: &RunToken,
    mut jobs: JobReceiver<AudioCommand>,
    wake_rx: UnixStream,
) -> anyhow::Result<()> {
    let mut proplist = Proplist::new().context("creating PulseAudio proplist")?;
    let _ = proplist.set_str(APPLICATION_NAME, "gpuishell");

    let mut mainloop = Mainloop::new().context("creating PulseAudio mainloop")?;

    // Declared after the mainloop so it is dropped before it.
    let _wake_event = mainloop
        .new_io_event(
            wake_rx.as_raw_fd(),
            IoEventFlagSet::INPUT,
            Box::new(move |_, _, _| {
                let mut buf = [0u8; 64];
                while matches!((&wake_rx).read(&mut buf), Ok(n) if n > 0) {}
            }),
        )
        .context("registering the PulseAudio wake-up event")?;

    let mut context = Context::new_with_proplist(&mainloop, "gpuishell", &proplist)
        .context("creating PulseAudio context")?;
    context
        .connect(None, FlagSet::NOFLAGS, None)
        .map_err(|e| anyhow!("connecting to PulseAudio: {e}"))?;

    // Wait for context to be ready
    loop {
        if !token.alive() {
            return Ok(());
        }
        iterate(&mut mainloop, true)?;
        match context.get_state() {
            context::State::Ready => break,
            context::State::Failed | context::State::Terminated => {
                bail!("PulseAudio context failed or terminated")
            }
            _ => {}
        }
    }

    debug!("PulseAudio connection established");
    token.active();

    let pending_queries = Rc::new(Cell::new(0u32));
    let local = Rc::new(RefCell::new(Local::default()));

    let query_sink = {
        let introspector = context.introspect();
        let local = local.clone();
        let pending = pending_queries.clone();
        move || {
            pending.set(pending.get() + 1);
            let local = local.clone();
            let pending = pending.clone();
            introspector.get_sink_info_by_name(DEFAULT_SINK, move |result| match result {
                ListResult::Item(sink) => {
                    let mut local = local.borrow_mut();
                    local.data.sink_volume = volume_to_percent(sink.volume.avg());
                    local.data.sink_muted = sink.mute;
                    local.sink = Some(sink.volume);
                }
                ListResult::End | ListResult::Error => {
                    pending.set(pending.get().saturating_sub(1));
                }
            });
        }
    };

    let query_source = {
        let introspector = context.introspect();
        let local = local.clone();
        let pending = pending_queries.clone();
        move || {
            pending.set(pending.get() + 1);
            let local = local.clone();
            let pending = pending.clone();
            introspector.get_source_info_by_name(DEFAULT_SOURCE, move |result| match result {
                ListResult::Item(source) => {
                    // Skip monitor sources (they mirror sinks)
                    if source.monitor_of_sink.is_none() {
                        let mut local = local.borrow_mut();
                        local.data.source_volume = volume_to_percent(source.volume.avg());
                        local.data.source_muted = source.mute;
                        local.source = Some(source.volume);
                    }
                }
                ListResult::End | ListResult::Error => {
                    pending.set(pending.get().saturating_sub(1));
                }
            });
        }
    };

    // Re-query on every sink, source or server change.
    let needs_refresh = Rc::new(Cell::new(true));
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
    let needs_refresh_cb = needs_refresh.clone();
    context.set_subscribe_callback(Some(Box::new(move |_facility, _operation, _idx| {
        needs_refresh_cb.set(true);
    })));

    let mut introspector = context.introspect();
    let mut published: Option<AudioData> = None;

    while token.alive() {
        if needs_refresh.replace(false) {
            query_sink();
            query_source();
        }

        jobs.drain(|command| apply(command, &mut introspector, &mut local.borrow_mut()));

        // Publish once every query in flight has answered, so a half-updated
        // snapshot never reaches the UI.
        if pending_queries.get() == 0 {
            let current = local.borrow().data.clone();
            if published.as_ref() != Some(&current) {
                debug!(
                    "Audio state: sink={}% (muted={}), source={}% (muted={})",
                    current.sink_volume,
                    current.sink_muted,
                    current.source_volume,
                    current.source_muted
                );
                token.write(data, |data| *data = current.clone());
                published = Some(current);
            }
        }

        iterate(&mut mainloop, !needs_refresh.get())?;
    }

    Ok(())
}

fn iterate(mainloop: &mut Mainloop, block: bool) -> anyhow::Result<()> {
    match mainloop.iterate(block) {
        IterateResult::Success(_) => Ok(()),
        IterateResult::Quit(_) | IterateResult::Err(_) => bail!("PulseAudio mainloop error"),
    }
}

/// Run one command against the default devices.
///
/// `local` is updated right away so the next queued command (another +5%
/// click) builds on this one instead of on the last snapshot.
fn apply(
    command: AudioCommand,
    introspector: &mut Introspector,
    local: &mut Local,
) -> anyhow::Result<()> {
    let step = |current: u8, delta: i8| (current as i16 + delta as i16).clamp(0, 100) as u8;
    let data = &local.data;
    let (sink, volume, toggle_mute) = match command {
        AudioCommand::SetSinkVolume(v) => (true, Some(v.min(100)), false),
        AudioCommand::SetSourceVolume(v) => (false, Some(v.min(100)), false),
        AudioCommand::AdjustSinkVolume(d) => (true, Some(step(data.sink_volume, d)), false),
        AudioCommand::AdjustSourceVolume(d) => (false, Some(step(data.source_volume, d)), false),
        AudioCommand::ToggleSinkMute => (true, None, true),
        AudioCommand::ToggleSourceMute => (false, None, true),
    };

    let (name, channels, percent, muted) = if sink {
        let data = &mut local.data;
        (
            DEFAULT_SINK,
            &mut local.sink,
            &mut data.sink_volume,
            &mut data.sink_muted,
        )
    } else {
        let data = &mut local.data;
        (
            DEFAULT_SOURCE,
            &mut local.source,
            &mut data.source_volume,
            &mut data.source_muted,
        )
    };

    if let Some(target) = volume {
        let channels = channels
            .as_mut()
            .ok_or_else(|| anyhow!("no {name} to set the volume of"))?;
        channels
            .scale(percent_to_volume(target))
            .ok_or_else(|| anyhow!("scaling the {name} volume"))?;
        if sink {
            introspector.set_sink_volume_by_name(name, channels, None);
        } else {
            introspector.set_source_volume_by_name(name, channels, None);
        }
        *percent = target;
    }

    if toggle_mute {
        *muted = !*muted;
        if sink {
            introspector.set_sink_mute_by_name(name, *muted, None);
        } else {
            introspector.set_source_mute_by_name(name, *muted, None);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_and_volume_round_trip() {
        for percent in [0, 1, 33, 50, 99, 100] {
            assert_eq!(volume_to_percent(percent_to_volume(percent)), percent);
        }
        assert_eq!(percent_to_volume(150), Volume::NORMAL);
    }
}
