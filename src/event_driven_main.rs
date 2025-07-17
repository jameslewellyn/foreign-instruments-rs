use tokio::sync::mpsc;
use log::{info, error, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::hid_devices::{HidDeviceManager, BasicHidEventHandler, HidEvent, process_hid_events};
// use crate::midi_output::MidiOutputManager; // Only import if actually used

// Event-driven application manager
pub struct EventDrivenApp {
    running: Arc<Mutex<bool>>,
}

impl EventDrivenApp {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            running: Arc::new(Mutex::new(true)),
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting event-driven Foreign Instruments Bridge...");

        // Create the event channel
        let (hid_event_sender, hid_event_receiver) = mpsc::unbounded_channel::<HidEvent>();
        
        // Create HID device manager with the sender
        let hid_manager = HidDeviceManager::new(hid_event_sender)?;
        
        // Create HID event handler (without receiver)
        let hid_handler = BasicHidEventHandler {};

        // Start HID device scanning
        hid_manager.scan_initial_devices()?;
        hid_manager.start_monitoring();

        // Spawn HID event processing task
        let hid_processing_task = tokio::spawn(async move {
            process_hid_events(Box::new(hid_handler), hid_event_receiver).await;
        });

        // Wait for HID processing task to complete
        tokio::select! {
            _ = hid_processing_task => {
                info!("HID processing task completed");
            }
        }

        Ok(())
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
}

// Main function for event-driven application
pub async fn run_event_driven_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // env_logger::init(); // Removed duplicate logger init
    
    info!("🎵 Starting Event-Driven Foreign Instruments Bridge");
    info!("==================================================");
    info!("Features:");
    info!("  ✅ HID device support (Maschine Jam, etc.)");
    info!("  ✅ Event-driven architecture (no polling)");
    info!("  ✅ Multi-threaded input/output");
    info!("  ✅ Real-time MIDI translation");
    info!("");

    let mut app = EventDrivenApp::new().await?;
    let running = Arc::new(Mutex::new(true));
    let running_clone = running.clone();
    
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        *running_clone.lock().unwrap() = false;
    })?;

    let app_task = tokio::spawn(async move {
        app.start().await
    });

    while *running.lock().unwrap() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Note: app is moved into the task, so we can't call stop() here
    // The application will stop when the task completes
    match app_task.await {
        Ok(Ok(())) => info!("Application stopped successfully"),
        Ok(Err(e)) => error!("Application error: {}", e),
        Err(e) => error!("Task error: {}", e),
    }

    info!("🎵 Foreign Instruments Bridge stopped");
    Ok(())
}

pub async fn test_hid_devices() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("🔍 Testing HID Device Detection");
    info!("================================");

    let (hid_event_sender, _hid_event_receiver) = mpsc::unbounded_channel::<HidEvent>();
    let hid_manager = HidDeviceManager::new(hid_event_sender)?;
    hid_manager.scan_initial_devices()?;
    let devices = hid_manager.get_devices();
    info!("Found {} HID devices:", devices.len());
    for device in &devices {
        info!("  🎹 {} ({:04x}:{:04x}) - {:?}", 
              device.name, device.vendor_id, device.product_id, device.state);
    }
    if devices.is_empty() {
        warn!("No HID devices found!");
    } else {
        info!("✅ HID device detection working!");
    }
    info!("");
    info!("Now you can test the event-driven architecture:");
    info!("1. Connect/disconnect your Maschine Jam");
    info!("2. Watch for HID events in the logs");
    info!("3. See MIDI messages generated from HID input");
    Ok(())
} 