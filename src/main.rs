/*

Automatically reconnect ASHA Bluetooth devices

Author:  John Schulz
Created: 31/12/2025

*/

use bluer::{Adapter, AdapterEvent, Address, DiscoveryFilter, Uuid, UuidExt};
use futures::StreamExt;
use mpris::{PlaybackStatus, PlayerFinder};
use std::{
    sync::atomic::Ordering::Relaxed,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
    usize,
};

const ASHA_SERVICE_U16: u16 = 0xFDF0;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let shared_play_state = Arc::new(AtomicBool::new(false));

    let filter = DiscoveryFilter {
        transport: bluer::DiscoveryTransport::Le,
        rssi: None,
        discoverable: false,
        duplicate_data: false,
        pattern: None,
        pathloss: None,
        ..Default::default()
    };

    let copy = Arc::clone(&shared_play_state);

    std::thread::spawn(move || monitor_playback(copy));

    loop {
        let Ok(session) = bluer::Session::new().await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Unable to get dbus session.");
            continue;
        };

        let Ok(adapter) = session.default_adapter().await else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("Unable to get default adapter.");
            continue;
        };

        let Ok(is_powered) = adapter.is_powered().await else {
            println!("Unable to get adapter state.");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };

        if !is_powered {
            println!("Adapter is off.");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let Ok(_) = adapter.set_discovery_filter(filter.clone()).await else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("Could not set discovery filter.");
            continue;
        };

        let Ok(discover_events) = adapter.discover_devices().await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Could noy start discovery.");
            continue;
        };

        println!("Discovering devices...");

        discover_events
            .for_each_concurrent(usize::MAX, |event| {
                handle_event(&shared_play_state, &adapter, event)
            })
            .await;
    }
}

fn monitor_playback(shared_bool: Arc<AtomicBool>) {
    let Ok(media_finder) = PlayerFinder::new() else {
        println!("Unable to get dbus session.");
        return;
    };

    loop {
        std::thread::sleep(Duration::from_millis(100));

        let Ok(player) = media_finder.find_active() else {
            shared_bool.store(false, std::sync::atomic::Ordering::Release);
            continue;
        };

        let Ok(playback_state) = player.get_playback_status() else {
            shared_bool.store(false, std::sync::atomic::Ordering::Release);
            continue;
        };

        shared_bool.store(
            playback_state == PlaybackStatus::Playing,
            std::sync::atomic::Ordering::Release,
        );
    }
}

async fn handle_event(playing: &Arc<AtomicBool>, adapter: &Adapter, event: AdapterEvent) {
    match event {
        AdapterEvent::DeviceAdded(address) => handle_device_added(playing, adapter, address).await,
        AdapterEvent::DeviceRemoved(address) => handle_device_removed(adapter, address).await,
        _ => return,
    }
}

async fn handle_device_added(playing: &Arc<AtomicBool>, adapter: &Adapter, address: Address) {
    let asha_profile = Uuid::from_u16(ASHA_SERVICE_U16);

    let Ok(device) = adapter.device(address) else {
        return;
    };

    let Ok(uuids) = device.uuids().await else {
        return;
    };

    let Some(uuid_list) = uuids else {
        return;
    };

    let has_asha = uuid_list
        .iter()
        .any(|uuid| uuid.as_u16() == Some(ASHA_SERVICE_U16));

    if !has_asha {
        // return;
    }

    let device_name = device
        .name()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string());

    if device_name != "SONNET 2" {
        return;
    }

    println!("ASHA device found: {}", device_name);

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let Ok(connected) = device.is_connected().await else {
            continue;
        };

        let Ok(trusted) = device.is_trusted().await else {
            continue;
        };

        if playing.load(Relaxed) && !connected {
            if !trusted {
                match device.set_trusted(true).await {
                    Ok(_) => println!("Trusted successfully."),
                    Err(_) => println!("Could not set device as trusted."),
                }
            }

            match device.connect_profile(&asha_profile).await {
                Ok(_) => println!("Connected successfully."),
                Err(_) => println!("Could not connect to device."),
            }
        } else if !playing.load(Relaxed) && connected {
            match device.disconnect().await {
                Ok(_) => println!("Disconnected successfully."),
                Err(_) => println!("Could not disconnect to device."),
            }
        }
    }
}

async fn handle_device_removed(adapter: &Adapter, address: Address) {
    let Ok(device) = adapter.device(address) else {
        return;
    };

    let device_name = device
        .name()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string());

    println!("Device removed: {}", device_name);
}

// async fn handle_device_change(device: &Device, property: DeviceProperty) {
//     let Ok(device_name) = device.name().await else {
//         return;
//     };

//     let adjusted_name = device_name.unwrap_or("Unknown".to_string());

//     println!("{:?} for {} changed...", property, adjusted_name);

//     match property {
//         DeviceProperty::ManufacturerData(_) => {}
//         DeviceProperty::Rssi(_) => {}
//         _ => return,
//     }

//     let Ok(is_connected) = device.is_connected().await else {
//         return;
//     };

//     if is_connected {
//         println!("{} already connected.", adjusted_name);
//         return;
//     }

//     let Ok(rssi) = device.rssi().await else {
//         return;
//     };

//     if rssi == None {
//         println!("RSSI is None, is the device off?");
//         return;
//     }

//     println!("Reconnecting device...");

//     // let asha_uuid = Uuid::from_u16(ASHA_SERVICE_U16);
//     // device.connect_profile(&asha_uuid)

//     match device.connect().await {
//         Ok(_) => println!("Successfully reconnected."),
//         Err(e) => println!("Failed with error: {}", e),
//     }
// }
