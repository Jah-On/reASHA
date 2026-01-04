/*

Automatically reconnect ASHA Bluetooth devices

Author:  John Schulz
Created: 31/12/2025

*/

use bluer::{DiscoveryFilter, Uuid, UuidExt};
use futures::{Stream, StreamExt};
use std::time::Duration;

const ASHA_SERVICE_U16: u16 = 0xFDF0;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        loop_fn().await;
    }
}

async fn loop_fn() {
    let filter = DiscoveryFilter {
        transport: bluer::DiscoveryTransport::Le,
        ..Default::default()
    };

    let asha_service_uuid = Uuid::from_u16(ASHA_SERVICE_U16);

    let Ok(session) = bluer::Session::new().await else {
        panic!("Unable to get dbus session!");
    };

    let Ok(adapter) = session.default_adapter().await else {
        tokio::time::sleep(Duration::from_mins(5)).await;
        return;
    };

    let Ok(is_powered) = adapter.is_powered().await else {
        tokio::time::sleep(Duration::from_mins(1)).await;
        return;
    };

    if !is_powered {
        tokio::time::sleep(Duration::from_mins(1)).await;
        return;
    }

    let Ok(_) = adapter.set_discovery_filter(filter).await else {
        println!("Could not set discovery filter.");
        return;
    };

    let Ok(_) = adapter.set_discoverable_timeout(1).await else {
        tokio::time::sleep(Duration::from_mins(5)).await;
        return;
    };

    let Ok(mut discover_events) = adapter.discover_devices().await else {
        tokio::time::sleep(Duration::from_mins(1)).await;
        return;
    };

    println!("Discovering devices...");

    while discover_events.size_hint().0 > 0 {
        let Some(event) = discover_events.next().await else {
            break;
        };

        let bluer::AdapterEvent::DeviceAdded(address) = event else {
            continue;
        };

        let Ok(device) = adapter.device(address) else {
            continue;
        };

        let Ok(uuids) = device.uuids().await else {
            continue;
        };

        let Some(uuid_list) = uuids else {
            continue;
        };

        let has_asha = uuid_list
            .iter()
            .any(|uuid| uuid.as_u16() == Some(ASHA_SERVICE_U16));

        if !has_asha {
            continue;
        }

        let device_name = device
            .name()
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "Unknown".to_string());

        println!("ASHA device found: {}", device_name);

        let Ok(is_connected) = device.is_connected().await else {
            println!("Could not check connection status");
            continue;
        };

        if is_connected {
            println!("Device is already connected");
            continue;
        }

        let Ok(_) = device.set_trusted(true).await else {
            println!("Could not set as trusted");
            continue;
        };

        println!("Reconnecting ...");

        // // Continue in loop if successful
        // let Err(_) = device.connect_profile(&asha_service_uuid).await else {
        //     println!("Successfully reconnected");
        //     continue;
        // };

        // let Ok(remote_address) = device.remote_address().await else {
        //     println!("Could not get remote address");
        //     continue;
        // };

        // println!("Trying alternate reconnect ...");

        // let Ok(device) = adapter.device(remote_address) else {
        //     println!("Could create device from remote address");
        //     continue;
        // };

        match device.connect_profile(&asha_service_uuid).await {
            Ok(_) => println!("Successfully reconnected"),
            Err(e) => println!("Failed to reconnect: {}", e),
        }
    }

    println!("Ending loop...");

    tokio::time::sleep(Duration::from_secs(10)).await;
}
