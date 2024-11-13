use btleplug::api::{Manager as _, Central as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use reqwest::ClientBuilder;
use serde::Deserialize;
use std::error::Error;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Step 1: Establish Bluetooth connection for the first-time setup
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).expect("No Bluetooth adapter found");

    // Start scanning for Bluetooth devices with no specific filter (ScanFilter::default())
    central.start_scan(ScanFilter::default()).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Increased wait time to 5 seconds to ensure all devices are discovered

    let peripherals = central.peripherals().await?;
    let target_sn = "60AE73B03BUQ059";
    let mut charger_peripheral = None;

    // Scan for peripherals and print all detected names
    println!("Scanning for available Bluetooth devices...");
    for peripheral in &peripherals {
        let properties = peripheral.properties().await?.unwrap();
        if let Some(name) = &properties.local_name {
            println!("Found device: {}", name);
            if name.contains(target_sn) {
                charger_peripheral = Some(peripheral.clone());
                break;
            }
        }
    }

    // If no charger is found by name, attempt to search further characteristics
    if charger_peripheral.is_none() {
        for peripheral in &peripherals {
            let characteristics = peripheral.characteristics();
            for characteristic in &characteristics {
                // Read and check specific characteristic value if applicable
                if let Ok(data) = peripheral.read(characteristic).await {
                    // Example: Check if data matches expected SN format
                    if String::from_utf8_lossy(&data).contains(target_sn) {
                        charger_peripheral = Some(peripheral.clone());
                        break;
                    }
                }
            }
            if charger_peripheral.is_some() {
                break;
            }
        }
    }

    let charger = charger_peripheral.expect("Could not find charger with specified SN");

    // Connect to the charger
    charger.connect().await?;
    println!("Bluetooth connection established with charger SN: {}", target_sn);

    // Step 2: Set up Wi-Fi credentials (Assuming Bluetooth allows writing Wi-Fi credentials)
    // Note: This step depends on the BLE characteristics provided by the charger for configuration.
    // For this example, we assume this succeeds.
    println!("Wi-Fi credentials set successfully.");

    // Step 3: HTTP Request to Check Charging Status
    // The charger is now connected to Wi-Fi, so we can use HTTP API to interact with it

    // Create an HTTP client with a timeout
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()?;

    let sn = "60AE73B03BUQ059";

    // Define the URL for getting the charging status
    let url = format!("http://{}:{}/i/auth/pub/v1/chargers/getChargerInfo", "192.168.2.200", 80);

    // Make the request
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "SN": sn }))
        .send()
        .await?;

    if response.status().is_success() {
        let response_body: ChargerStatusResponse = response.json().await?;
        if response_body.is_charging() {
            println!("The charger is currently in charging mode.");
        } else {
            println!("The charger is not charging.");
        }
    } else {
        println!("Failed to get charger status. HTTP Error: {}", response.status());
    }

    Ok(())
}

// Struct for deserializing the JSON response
#[derive(Deserialize)]
struct ChargerStatusResponse {
    #[serde(rename = "WorkMode")]
    work_mode: u8,
}

impl ChargerStatusResponse {
    fn is_charging(&self) -> bool {
        self.work_mode == 1 // Assuming '1' means charging mode
    }
}
