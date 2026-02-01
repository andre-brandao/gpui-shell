use super::Network;
use super::types::*;
use crate::services::ServiceEvent;
use anyhow::Result;
use futures_lite::StreamExt;
use std::sync::mpsc;
use zbus::proxy;

// NetworkManager D-Bus proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_wireless_enabled(&self, enabled: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

pub async fn run_listener(tx: &mpsc::Sender<ServiceEvent<Network>>) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let nm = NetworkManagerProxy::new(&conn).await?;

    // Send initial state
    let data = fetch_network_data(&nm).await?;
    let _ = tx.send(ServiceEvent::Update(NetworkEvent::StateChanged(data)));

    // Listen for property changes
    let mut wifi_stream = nm.receive_wireless_enabled_changed().await;
    let mut connectivity_stream = nm.receive_connectivity_changed().await;
    let mut state_stream = nm.receive_state_changed().await;

    loop {
        tokio::select! {
            Some(_) = async { wifi_stream.next().await } => {
                if let Ok(data) = fetch_network_data(&nm).await {
                    let _ = tx.send(ServiceEvent::Update(NetworkEvent::StateChanged(data)));
                }
            }
            Some(_) = async { connectivity_stream.next().await } => {
                if let Ok(data) = fetch_network_data(&nm).await {
                    let _ = tx.send(ServiceEvent::Update(NetworkEvent::StateChanged(data)));
                }
            }
            Some(_) = async { state_stream.next().await } => {
                if let Ok(data) = fetch_network_data(&nm).await {
                    let _ = tx.send(ServiceEvent::Update(NetworkEvent::StateChanged(data)));
                }
            }
        }
    }
}

async fn fetch_network_data(nm: &NetworkManagerProxy<'_>) -> Result<NetworkData> {
    let wifi_enabled = nm.wireless_enabled().await.unwrap_or(false);
    let connectivity = match nm.connectivity().await.unwrap_or(0) {
        0 => ConnectivityState::Unknown,
        1 => ConnectivityState::None,
        2 => ConnectivityState::Portal,
        3 => ConnectivityState::Limited,
        4 => ConnectivityState::Full,
        _ => ConnectivityState::Unknown,
    };

    Ok(NetworkData {
        wifi_enabled,
        connectivity,
        wifi_present: true, // TODO: detect actual wifi hardware
        ..Default::default()
    })
}

pub async fn execute_command(command: NetworkCommand) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let nm = NetworkManagerProxy::new(&conn).await?;

    match command {
        NetworkCommand::ToggleWiFi => {
            let current = nm.wireless_enabled().await?;
            nm.set_wireless_enabled(!current).await?;
        }
        NetworkCommand::ToggleAirplaneMode => {
            // Toggle WiFi as part of airplane mode
            let current = nm.wireless_enabled().await?;
            nm.set_wireless_enabled(!current).await?;
            // TODO: Also toggle Bluetooth via rfkill
        }
        NetworkCommand::ScanNearByWiFi => {
            // TODO: Implement WiFi scanning
            log::info!("WiFi scan requested");
        }
        NetworkCommand::SelectAccessPoint { ap, password } => {
            // TODO: Implement access point connection
            log::info!(
                "Connect to AP: {} (password: {})",
                ap.ssid,
                password.is_some()
            );
        }
        NetworkCommand::ToggleVpn(vpn) => {
            // TODO: Implement VPN toggling
            log::info!("Toggle VPN: {}", vpn.name);
        }
    }

    Ok(())
}
