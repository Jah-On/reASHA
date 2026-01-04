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
    loop {
        loop_fn().await;
    }
}

async fn loop_fn() {
    let filter = DiscoveryFilter {
        transport: bluer::DiscoveryTransport::Le,
        rssi: None,
        discoverable: false,
        duplicate_data: false,
        pattern: None,
        pathloss: None,
        ..Default::default()
    };

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

    let Ok(discover_events) = adapter.discover_devices().await else {
        tokio::time::sleep(Duration::from_mins(1)).await;
        return;
    };

    println!("Discovering devices...");

    discover_events
        .for_each(|event| handle_event(&adapter, event))
        .await;

    // while discover_events.size_hint().0 > 0 {
    //     let Some(event) = discover_events.next().await else {
    //         break;
    //     };

    // }

    // println!("Ending loop...");

    // tokio::time::sleep(Duration::from_secs(20)).await;
}

async fn handle_event(adapter: &Adapter, event: AdapterEvent) {
    match event {
        AdapterEvent::DeviceAdded(address) => handle_device_added(adapter, address).await,
        AdapterEvent::DeviceRemoved(address) => handle_device_removed(adapter, address).await,
        _ => return,
    }

    // println!("Reconnecting ...");

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

    // match device.connect_profile(&asha_service_uuid).await {
    //     Ok(_) => println!("Successfully reconnected"),
    //     Err(e) => println!("Failed to reconnect: {}", e),
    // }
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
