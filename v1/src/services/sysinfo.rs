use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use std::ops::Deref;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, System};

/// Network speed and IP data.
#[derive(Debug, Clone, Default)]
pub struct NetworkData {
    pub ip: Option<String>,
    pub download_speed: u32, // KB/s
    pub upload_speed: u32,   // KB/s
}

/// Disk usage data.
#[derive(Debug, Clone)]
pub struct DiskData {
    pub mount_point: String,
    pub usage_percent: u32,
    pub total_gb: f64,
    pub used_gb: f64,
}

/// System information data.
#[derive(Debug, Clone, Default)]
pub struct SysInfoData {
    pub cpu_usage: u32,
    pub memory_usage: u32,
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub swap_usage: u32,
    pub temperature: Option<i32>,
    pub disks: Vec<DiskData>,
    pub network: NetworkData,
}

/// Events from SysInfo service.
#[derive(Debug, Clone)]
pub enum SysInfoEvent {
    StateChanged(SysInfoData),
}

/// SysInfo service for system monitoring.
#[derive(Debug, Clone, Default)]
pub struct SysInfo {
    pub data: SysInfoData,
}

impl Deref for SysInfo {
    type Target = SysInfoData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for SysInfo {
    type UpdateEvent = SysInfoEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            SysInfoEvent::StateChanged(data) => {
                self.data = data;
            }
        }
    }
}

impl SysInfo {
    /// Create a new GPUI Entity for the SysInfo service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<SysInfo>>();

        // Spawn a polling thread for system info
        std::thread::spawn(move || {
            let mut system = System::new();
            let mut components = Components::new_with_refreshed_list();
            let mut disks = Disks::new_with_refreshed_list();
            let mut networks = Networks::new_with_refreshed_list();
            let mut last_check: Option<Instant> = None;
            let mut last_received: u64 = 0;
            let mut last_transmitted: u64 = 0;

            loop {
                let data = fetch_system_info(
                    &mut system,
                    &mut components,
                    &mut disks,
                    &mut networks,
                    &mut last_check,
                    &mut last_received,
                    &mut last_transmitted,
                );
                let _ = tx.send(ServiceEvent::Update(SysInfoEvent::StateChanged(data)));
                std::thread::sleep(Duration::from_secs(2));
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
                                ServiceEvent::Init(sysinfo) => {
                                    this.data = sysinfo.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("SysInfo service error: {}", e);
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
                    .timer(Duration::from_millis(500))
                    .await;
            }
        })
        .detach();

        SysInfo {
            data: SysInfoData::default(),
        }
    }

    /// Get CPU usage as a formatted string.
    pub fn cpu_str(&self) -> String {
        format!("{}%", self.data.cpu_usage)
    }

    /// Get memory usage as a formatted string.
    pub fn memory_str(&self) -> String {
        format!("{}%", self.data.memory_usage)
    }

    /// Get memory usage with details.
    pub fn memory_details_str(&self) -> String {
        format!(
            "{:.1}/{:.1} GB ({}%)",
            self.data.memory_used_gb, self.data.memory_total_gb, self.data.memory_usage
        )
    }

    /// Get temperature as a formatted string.
    pub fn temperature_str(&self) -> Option<String> {
        self.data.temperature.map(|t| format!("{}°C", t))
    }

    /// Get download speed as a formatted string.
    pub fn download_str(&self) -> String {
        format_speed(self.data.network.download_speed)
    }

    /// Get upload speed as a formatted string.
    pub fn upload_str(&self) -> String {
        format_speed(self.data.network.upload_speed)
    }
}

fn format_speed(kb_per_sec: u32) -> String {
    if kb_per_sec >= 1000 {
        format!("{} MB/s", kb_per_sec / 1000)
    } else {
        format!("{} KB/s", kb_per_sec)
    }
}

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
    components.refresh();
    disks.refresh();
    networks.refresh();

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
        .map(|c| c.temperature() as i32);

    let disk_data: Vec<DiskData> = disks
        .iter()
        .filter(|d| !d.is_removable() && d.total_space() > 0)
        .map(|d| {
            let total = d.total_space() as f64;
            let available = d.available_space() as f64;
            let used = total - available;
            DiskData {
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

    for (name, data) in networks.iter() {
        if name.contains("en")
            || name.contains("eth")
            || name.contains("wl")
            || name.contains("wlan")
        {
            total_received += data.received();
            total_transmitted += data.transmitted();
            if ip.is_none() {
                ip = data
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
        network: NetworkData {
            ip: ip,
            download_speed,
            upload_speed,
        },
    }
}
