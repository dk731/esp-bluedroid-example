use std::time::Duration;

use anyhow;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter, WriteType};
use btleplug::platform::Manager;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tokio::time;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct LedConfiguration {
    pwm_duty: f32,
    pwm_frequency: f32,
    enabled: bool,
}

lazy_static! {
    pub static ref TOKIO_RUNTIME: tokio::runtime::Runtime = {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.worker_threads(4);
        builder.enable_all();
        builder.build().unwrap()
    };
    static ref MANAGER: btleplug::platform::Manager =
        TOKIO_RUNTIME.block_on(Manager::new()).unwrap();
    static ref ADAPTER_LIST: Vec<btleplug::platform::Adapter> =
        TOKIO_RUNTIME.block_on(MANAGER.adapters()).unwrap();
    pub static ref ADAPTER: &'static btleplug::platform::Adapter = &ADAPTER_LIST[0];
    pub static ref ESP_PERIPHERAL: RwLock<Option<btleplug::platform::Peripheral>> =
        RwLock::new(None);
    pub static ref APP_HANDLE: RwLock<Option<AppHandle>> = RwLock::new(None);
}

const ESP_BLE_NAME: &str = "esp-bluedroid LED Example";

pub async fn ble_monitoring() -> anyhow::Result<()> {
    println!("Starting BLE monitoring...");

    loop {
        time::sleep(Duration::from_secs(3)).await;

        if ESP_PERIPHERAL.read().await.is_some() {
            println!("ESP peripheral is already connected, checking connection status...");

            let is_connected = ESP_PERIPHERAL
                .read()
                .await
                .as_ref()
                .unwrap()
                .is_connected()
                .await?;

            if is_connected {
                println!("ESP peripheral is connected, skipping scan...");
                APP_HANDLE
                    .read()
                    .await
                    .as_ref()
                    .unwrap()
                    .emit("connection-status", true)?;
            } else {
                println!("Sending event of disconnected");
                *ESP_PERIPHERAL.write().await = None;

                APP_HANDLE
                    .read()
                    .await
                    .as_ref()
                    .unwrap()
                    .emit("connection-status", false)?;
            }

            continue;
        }

        println!("Starting scan on {}...", ADAPTER.adapter_info().await?);
        ADAPTER
            .start_scan(ScanFilter::default())
            .await
            .expect("Can't scan BLE adapter for connected devices...");
        time::sleep(Duration::from_secs(1)).await;

        let peripherals = ADAPTER.peripherals().await?;
        let mut found = false;

        for peripheral in peripherals.iter() {
            let properties = peripheral.properties().await?;
            let is_connected = peripheral.is_connected().await?;
            let local_name = properties
                .unwrap()
                .local_name
                .unwrap_or(String::from("(peripheral name unknown)"));

            if local_name == ESP_BLE_NAME {
                found = true;

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
                *ESP_PERIPHERAL.write().await = Some(peripheral.clone());

                if is_connected {
                    println!("Sending event of connected");
                    APP_HANDLE
                        .read()
                        .await
                        .as_ref()
                        .unwrap()
                        .emit("connection-status", true)?;
                }
            }
        }

        if !found {
            *ESP_PERIPHERAL.write().await = None;

            println!("Sending event of disconnected");
            APP_HANDLE
                .read()
                .await
                .as_ref()
                .unwrap()
                .emit("connection-status", false)?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn update_led_config(led_config: LedConfiguration) {
    println!("Updating LED configuration: {:?}", led_config);

    let esp_lock = ESP_PERIPHERAL.read().await;
    let Some(esp) = esp_lock.as_ref() else {
        println!("ESP peripheral not found!");
        return;
    };
    if let Err(err) = esp.discover_services().await {
        println!("Error discovering services: {}", err);
        return;
    }

    let characteristics = esp.characteristics();
    let Some(led_config_char) = characteristics
        .iter()
        .find(|el| el.uuid == Uuid::from_u128(42424242))
    else {
        println!("LED configuration characteristic not found!");
        return;
    };

    let Ok(new_config_bytes) =
        bincode::serde::encode_to_vec(led_config, bincode::config::standard())
    else {
        println!("Failed to serialize LED configuration!");
        return;
    };
    esp.write(
        led_config_char,
        &new_config_bytes,
        WriteType::WithoutResponse,
    )
    .await
    .unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    lazy_static::initialize(&ADAPTER);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![update_led_config])
        .setup(|app| {
            TOKIO_RUNTIME.block_on(async {
                *APP_HANDLE.write().await = Some(app.handle().clone());
            });

            TOKIO_RUNTIME.spawn(ble_monitoring());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
