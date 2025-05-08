use std::time::Duration;

use anyhow;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use lazy_static::lazy_static;
use tokio::time;

lazy_static! {
    static ref TOKIO_RUNTIME: tokio::runtime::Runtime = {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.worker_threads(4);
        builder.enable_all();
        builder.build().unwrap()
    };
    static ref MANAGER: btleplug::platform::Manager =
        TOKIO_RUNTIME.block_on(Manager::new()).unwrap();
    static ref ADAPTER_LIST: Vec<btleplug::platform::Adapter> =
        TOKIO_RUNTIME.block_on(MANAGER.adapters()).unwrap();
    static ref ADAPTER: &'static btleplug::platform::Adapter = &ADAPTER_LIST[0];
}

pub async fn ble_monitoring() -> anyhow::Result<()> {
    log::info!("Starting BLE monitoring...");

    loop {
        log::info!("Starting scan on {}...", ADAPTER.adapter_info().await?);
        ADAPTER
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(1)).await;

        let peripherals = ADAPTER.peripherals().await?;

        for peripheral in peripherals.iter() {
            let properties = peripheral.properties().await?;
            let is_connected = peripheral.is_connected().await?;
            let local_name = properties
                .unwrap()
                .local_name
                .unwrap_or(String::from("(peripheral name unknown)"));
            log::info!(
                "Peripheral {:?} is connected: {:?}",
                local_name,
                is_connected
            );
        }
    }

    Ok(())
}

async fn ble_test() -> anyhow::Result<()> {
    println!("Starting BLE test...");

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }
    println!("Found {} Bluetooth adapters", adapter_list.len());

    let adapter = &adapter_list[0];

    println!("Starting scan on {}...", adapter.adapter_info().await?);
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");
    time::sleep(Duration::from_secs(1)).await;

    let peripherals = adapter.peripherals().await?;

    for peripheral in peripherals.iter() {
        let properties = peripheral.properties().await?;
        let is_connected = peripheral.is_connected().await?;
        let local_name = properties
            .unwrap()
            .local_name
            .unwrap_or(String::from("(peripheral name unknown)"));
        println!(
            "Peripheral {:?} is connected: {:?}",
            local_name, is_connected
        );
        if !is_connected {
            println!("Connecting to peripheral {:?}...", &local_name);
            if let Err(err) = peripheral.connect().await {
                eprintln!("Error connecting to peripheral, skipping: {}", err);
                continue;
            }
        }
        let is_connected = peripheral.is_connected().await?;
        println!(
            "Now connected ({:?}) to peripheral {:?}...",
            is_connected, &local_name
        );
        peripheral.discover_services().await?;
        println!("Discover peripheral {:?} services...", &local_name);
        for service in peripheral.services() {
            println!(
                "Service UUID {}, primary: {}",
                service.uuid, service.primary
            );
            for characteristic in service.characteristics {
                println!("  {:?}", characteristic);
            }
        }
        if is_connected {
            println!("Disconnecting from peripheral {:?}...", &local_name);
            peripheral
                .disconnect()
                .await
                .expect("Error disconnecting from BLE peripheral");
        }
    }

    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    tokio::runtime::Runtime::new()
        .expect("Failed to create runtime")
        .block_on(ble_test())
        .expect("Failed to run BLE test");

    format!("Hello, {}! You've been greeted from Rust! !!!!!!####", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
