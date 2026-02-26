//! System information service for monitoring CPU, memory, disk, and network.
//!
//! This module provides a reactive subscriber for monitoring system resources
//! using the sysinfo crate.

use std::thread;
use std::time::{Duration, Instant};

use futures_signals::signal::{Mutable, MutableSignalCloned};
use sysinfo::{Components, Disks, Networks, System};
use tracing::debug;

use crate::ServiceStatus;

/// Network speed and IP data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkInfo {
    /// Local IP address.
    pub ip: Option<String>,
    /// Download speed in KB/s.
    pub download_speed: u32,
    /// Upload speed in KB/s.
    pub upload_speed: u32,
}

impl NetworkInfo {
    /// Format download speed as human-readable string.
    pub fn download_str(&self) -> String {
        format_speed(self.download_speed)
    }

    /// Format upload speed as human-readable string.
    pub fn upload_str(&self) -> String {
        format_speed(self.upload_speed)
    }
}

/// Disk usage data.
#[derive(Debug, Clone, PartialEq)]
pub struct DiskInfo {
    /// Mount point path.
    pub mount_point: String,
    /// Usage percentage (0-100).
    pub usage_percent: u32,
    /// Total space in GB.
    pub total_gb: f64,
    /// Used space in GB.
    pub used_gb: f64,
}

impl DiskInfo {
    /// Format as human-readable string.
    pub fn usage_str(&self) -> String {
        format!(
            "{:.1}/{:.1} GB ({}%)",
            self.used_gb, self.total_gb, self.usage_percent
        )
    }
}

/// System information data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SysInfoData {
    /// CPU usage percentage (0-100).
    pub cpu_usage: u32,
    /// Memory usage percentage (0-100).
    pub memory_usage: u32,
    /// Total memory in GB.
    pub memory_total_gb: f64,
    /// Used memory in GB.
    pub memory_used_gb: f64,
    /// Swap usage percentage (0-100).
    pub swap_usage: u32,
    /// CPU temperature in Celsius.
    pub temperature: Option<i32>,
    /// Disk information.
    pub disks: Vec<DiskInfo>,
    /// Network information.
    pub network: NetworkInfo,
}

impl SysInfoData {
    /// Get CPU usage as a formatted string.
    pub fn cpu_str(&self) -> String {
        format!("{}%", self.cpu_usage)
    }

    /// Get memory usage as a formatted string.
    pub fn memory_str(&self) -> String {
        format!("{}%", self.memory_usage)
    }

    /// Get memory usage with details.
    pub fn memory_details_str(&self) -> String {
        format!(
            "{:.1}/{:.1} GB ({}%)",
            self.memory_used_gb, self.memory_total_gb, self.memory_usage
        )
    }

    /// Get temperature as a formatted string.
    pub fn temperature_str(&self) -> Option<String> {
        self.temperature.map(|t| format!("{}°C", t))
    }

    /// Get an icon for CPU usage.
    pub fn cpu_icon(&self) -> &'static str {
        match self.cpu_usage {
            0..=25 => "󰻠",
            26..=50 => "󰻠",
            51..=75 => "󰻠",
            _ => "󰻠",
        }
    }

    /// Get an icon for memory usage.
    pub fn memory_icon(&self) -> &'static str {
        "󰍛"
    }

    /// Get an icon for temperature.
    pub fn temperature_icon(&self) -> &'static str {
        match self.temperature {
            Some(t) if t >= 80 => "󰸁", // Hot
            Some(t) if t >= 60 => "󱃃", // Warm
            _ => "󱃂",                  // Normal
        }
    }
}

/// Event-driven system information subscriber.
///
/// This subscriber monitors system resources via sysinfo
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct SysInfoSubscriber {
    data: Mutable<SysInfoData>,
    status: Mutable<ServiceStatus>,
}

impl SysInfoSubscriber {
    /// Create a new sysinfo subscriber and start monitoring.
    ///
    /// Polls system information every 2 seconds.
    pub fn new() -> Self {
        let data = Mutable::new(SysInfoData::default());
        let status = Mutable::new(ServiceStatus::Active);

        // Start the polling listener
        start_listener(data.clone(), status.clone());

        Self { data, status }
    }

    /// Get a signal that emits when system info changes.
    pub fn subscribe(&self) -> MutableSignalCloned<SysInfoData> {
        self.data.signal_cloned()
    }

    /// Get the current system info snapshot.
    pub fn get(&self) -> SysInfoData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }
}

impl Default for SysInfoSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// Format speed in KB/s or MB/s.
fn format_speed(kb_per_sec: u32) -> String {
    if kb_per_sec >= 1000 {
        format!("{} MB/s", kb_per_sec / 1000)
    } else {
        format!("{} KB/s", kb_per_sec)
    }
}

/// Start the polling listener thread.
fn start_listener(data: Mutable<SysInfoData>, _status: Mutable<ServiceStatus>) {
    thread::spawn(move || {
        let mut system = System::new();
        let mut components = Components::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut networks = Networks::new_with_refreshed_list();
        let mut last_check: Option<Instant> = None;
        let mut last_received: u64 = 0;
        let mut last_transmitted: u64 = 0;

        loop {
            let new_data = fetch_system_info(
                &mut system,
                &mut components,
                &mut disks,
                &mut networks,
                &mut last_check,
                &mut last_received,
                &mut last_transmitted,
            );

            // Only update if changed
            let current = data.lock_ref().clone();
            if new_data != current {
                debug!(
                    "SysInfo updated: CPU {}%, MEM {}%, TEMP {:?}°C",
                    new_data.cpu_usage, new_data.memory_usage, new_data.temperature
                );
                *data.lock_mut() = new_data;
            }

            thread::sleep(Duration::from_secs(2));
        }
    });
}

/// Fetch current system information.
fn fetch_system_info(
    system: &mut System,
    components: &mut Components,
    disks: &mut Disks,
    networks: &mut Networks,
    last_check: &mut Option<Instant>,
    last_received: &mut u64,
    last_transmitted: &mut u64,
) -> SysInfoData {
    system.refresh_memory();
    system.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
    components.refresh(true);
    disks.refresh(true);
    networks.refresh(true);

    let cpu_usage = system.global_cpu_usage().floor() as u32;

    let total_memory = system.total_memory() as f64;
    let available_memory = system.available_memory() as f64;
    let used_memory = total_memory - available_memory;
    let memory_usage = ((used_memory / total_memory) * 100.0) as u32;
    let memory_total_gb = total_memory / 1_073_741_824.0; // bytes to GB
    let memory_used_gb = used_memory / 1_073_741_824.0;

    let total_swap = system.total_swap() as f64;
    let free_swap = system.free_swap() as f64;
    let swap_usage = if total_swap > 0.0 {
        (((total_swap - free_swap) / total_swap) * 100.0) as u32
    } else {
        0
    };

    // Try common temperature sensors
    let temperature = components
        .iter()
        .find(|c| {
            let label = c.label().to_lowercase();
            label.contains("coretemp")
                || label.contains("k10temp")
                || label.contains("cpu")
                || label.contains("package")
        })
        .and_then(|c| c.temperature().map(|t| t as i32));

    let disk_data: Vec<DiskInfo> = disks
        .iter()
        .filter(|d| !d.is_removable() && d.total_space() > 0)
        .map(|d| {
            let total = d.total_space() as f64;
            let available = d.available_space() as f64;
            let used = total - available;
            DiskInfo {
                mount_point: d.mount_point().to_string_lossy().to_string(),
                usage_percent: ((used / total) * 100.0) as u32,
                total_gb: total / 1_073_741_824.0,
                used_gb: used / 1_073_741_824.0,
            }
        })
        .collect();

    // Calculate network speeds
    let elapsed_secs = last_check.map(|t| t.elapsed().as_secs()).unwrap_or(0);

    let mut total_received: u64 = 0;
    let mut total_transmitted: u64 = 0;
    let mut ip: Option<String> = None;

    for (name, net_data) in networks.iter() {
        if name.contains("en")
            || name.contains("eth")
            || name.contains("wl")
            || name.contains("wlan")
        {
            total_received += net_data.received();
            total_transmitted += net_data.transmitted();
            if ip.is_none() {
                ip = net_data
                    .ip_networks()
                    .iter()
                    .find(|ipn| ipn.addr.is_ipv4())
                    .map(|ipn| ipn.addr.to_string());
            }
        }
    }

    let (download_speed, upload_speed) = if elapsed_secs > 0 && *last_received > 0 {
        let recv_diff = total_received.saturating_sub(*last_received);
        let trans_diff = total_transmitted.saturating_sub(*last_transmitted);
        (
            (recv_diff / 1000 / elapsed_secs) as u32,
            (trans_diff / 1000 / elapsed_secs) as u32,
        )
    } else {
        (0, 0)
    };

    *last_check = Some(Instant::now());
    *last_received = total_received;
    *last_transmitted = total_transmitted;

    SysInfoData {
        cpu_usage,
        memory_usage,
        memory_total_gb,
        memory_used_gb,
        swap_usage,
        temperature,
        disks: disk_data,
        network: NetworkInfo {
            ip,
            download_speed,
            upload_speed,
        },
    }
}
