/*

Automatically reconnect ASHA Bluetooth devices

Author:  John Schulz
Created: 31/12/2025

*/

use bluer::{
    Adapter,
    AdapterEvent,
    Address,
    Device,
    DeviceEvent,
    DeviceProperty,
    DiscoveryFilter,
    // Uuid,
    UuidExt,
};
use futures::StreamExt;
use std::time::Duration;

const ASHA_SERVICE_U16: u16 = 0xFDF0;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let filter = DiscoveryFilter {
        transport: bluer::DiscoveryTransport::Le,
        rssi: None,
        discoverable: false,
        duplicate_data: false,
        pattern: None,
        pathloss: None,
        ..Default::default()
    };

    loop {
        let Ok(session) = bluer::Session::new().await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Unable to get dbus session.");
            continue;
        };

        let Ok(adapter) = session.default_adapter().await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Unable to get default adapter.");
            continue;
        };

        let Ok(is_powered) = adapter.is_powered().await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Unable to get adapter state.");
            continue;
        };

        if !is_powered {
            tokio::time::sleep(Duration::from_mins(1)).await;
            println!("Adapter is off.");
            continue;
        }

        let Ok(_) = adapter.set_discovery_filter(filter.clone()).await else {
            tokio::time::sleep(Duration::from_mins(1)).await;
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
            .for_each(|event| handle_event(&adapter, event))
            .await;
    }
}

async fn handle_event(adapter: &Adapter, event: AdapterEvent) {
    match event {
        AdapterEvent::DeviceAdded(address) => handle_device_added(adapter, address).await,
        AdapterEvent::DeviceRemoved(address) => handle_device_removed(adapter, address).await,
        _ => return,
    }
}

async fn handle_device_added(adapter: &Adapter, address: Address) {
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
        return;
    }

    let device_name = device
        .name()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string());

    // if device_name != "SONNET 2" {
    //     return;
    // }

    println!("ASHA device found: {}", device_name);

    let Ok(device_events) = device.events().await else {
        return;
    };

    device_events
        .for_each(|DeviceEvent::PropertyChanged(event)| handle_device_change(&device, event))
        .await;
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

async fn handle_device_change(device: &Device, property: DeviceProperty) {
    let Ok(device_name) = device.name().await else {
        return;
    };

    let adjusted_name = device_name.unwrap_or("Unknown".to_string());

    println!("{:?} for {} changed...", property, adjusted_name);

    match property {
        DeviceProperty::ManufacturerData(_) => {}
        DeviceProperty::Rssi(_) => {}
        _ => return,
    }

    let Ok(is_connected) = device.is_connected().await else {
        return;
    };

    if is_connected {
        println!("{} already connected.", adjusted_name);
        return;
    }

    let Ok(rssi) = device.rssi().await else {
        return;
    };

    if rssi == None {
        println!("RSSI is None, is the device off?");
        return;
    }

    println!("Reconnecting device...");

    // let asha_uuid = Uuid::from_u16(ASHA_SERVICE_U16);
    // device.connect_profile(&asha_uuid)

    match device.connect().await {
        Ok(_) => println!("Successfully reconnected."),
        Err(e) => println!("Failed with error: {}", e),
    }
}
