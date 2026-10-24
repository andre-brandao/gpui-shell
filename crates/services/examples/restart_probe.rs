//! Restart every service repeatedly and report threads and fds, to catch a
//! listener that outlives its run.
//!
//! `cargo run -p services --example restart_probe [rounds]`
//!
//! Tray and notifications are left out: they claim session-bus names a
//! running shell already owns.

use std::time::Duration;

use services::{
    AudioSubscriber, BluetoothSubscriber, BrightnessSubscriber, ManagedService, MprisSubscriber,
    NetworkSubscriber, PrivacySubscriber, SysInfoSubscriber, UPowerSubscriber,
};

fn count(dir: &str) -> usize {
    std::fs::read_dir(dir).map_or(0, |entries| entries.count())
}

fn report(label: &str, services: &[Box<dyn ManagedService>]) {
    println!(
        "{label:>14}: threads={:>3} fds={:>3}",
        count("/proc/self/task"),
        count("/proc/self/fd")
    );
    for service in services {
        println!("{:>16} {:?}", service.name(), service.status());
    }
}

#[tokio::main]
async fn main() {
    let rounds: usize = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(50);

    let services: Vec<Box<dyn ManagedService>> = vec![
        Box::new(AudioSubscriber::new()),
        Box::new(BluetoothSubscriber::new()),
        Box::new(BrightnessSubscriber::new()),
        Box::new(MprisSubscriber::new()),
        Box::new(NetworkSubscriber::new()),
        Box::new(PrivacySubscriber::new()),
        Box::new(SysInfoSubscriber::new()),
        Box::new(UPowerSubscriber::new()),
    ];

    for service in &services {
        service.start();
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    report("started", &services);

    for _ in 0..rounds {
        for service in &services {
            service.restart();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    report(&format!("{rounds} restarts"), &services);

    for service in &services {
        service.stop();
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    report("stopped", &services);
}
