/*

Automatically reconnect ASHA Bluetooth devices

Author:  John Schulz
Created: 31/12/2025

*/

use std::time::{self, Duration};
use futures::{StreamExt};
use bluer::UuidExt;

const ASHA_SERVICE_UUID: u16 = 0xFDF0;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        loop_fn().await;
    }
}

async fn loop_fn(){
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

    let Ok(mut discover_events) = adapter.discover_devices().await else {
        tokio::time::sleep(Duration::from_mins(1)).await;
        return;
    };

    let Ok(_) = adapter.set_discoverable_timeout(1).await else {
        tokio::time::sleep(Duration::from_mins(5)).await;
        return;
    };

    println!("Discovering devices...");

    let time_point = time::Instant::now();

    while (time::Instant::now() - time_point) < Duration::from_secs(1) {
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

        let has_asha = uuid_list.iter().any(|uuid| {
            uuid.as_u16() == Some(ASHA_SERVICE_UUID)
        });

        if !has_asha {
            continue;
        }

        let device_name = device.name().await.ok().flatten().unwrap_or_else(|| "Unknown".to_string());
        println!("ASHA device found: {}", device_name);
    
        let Ok(is_connected) = device.is_connected().await else {
            println!("Could not check connection status");
            continue;
        };

        if !is_connected {
            println!("Reconnecting ...");
            match device.connect().await {
                Ok(_) => println!("Successfully reconnected"),
                Err(e) => println!("Failed to reconnect: {}", e),
            }
        } else {
            println!("Device is already connected");
        }
    }

    println!("Ending loop...");

    tokio::time::sleep(Duration::from_secs(10)).await;
}