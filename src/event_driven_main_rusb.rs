use tokio::sync::mpsc;
use log::{info, error, warn};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::rusb_hid_manager::{RusbHidManager, BasicRusbHidEventHandler, RusbHidEvent, process_rusb_hid_events};

// Event-driven application manager using rusb
pub struct EventDrivenRusbApp {
    running: Arc<Mutex<bool>>,
}

impl EventDrivenRusbApp {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            running: Arc::new(Mutex::new(true)),
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting event-driven Foreign Instruments Bridge (rusb)...");

        // Create the event channel
        let (rusb_event_sender, rusb_event_receiver) = mpsc::unbounded_channel::<RusbHidEvent>();
        
        // Create rusb HID device manager with the sender
        let rusb_manager = RusbHidManager::new(rusb_event_sender)?;
        
        // Create rusb HID event handler
        let rusb_handler = BasicRusbHidEventHandler {};

        // Start USB device scanning
        rusb_manager.scan_initial_devices()?;
        rusb_manager.start_monitoring();

        // Spawn rusb HID event processing task
        let rusb_processing_task = tokio::spawn(async move {
            process_rusb_hid_events(rusb_handler, rusb_event_receiver).await;
        });

        // Wait for rusb HID processing task to complete
        tokio::select! {
            _ = rusb_processing_task => {
                info!("Rusb HID processing task completed");
            }
        }

        Ok(())
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
}

// Main function for event-driven application using rusb
pub async fn run_event_driven_rusb_app() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // env_logger::init(); // Removed duplicate logger init
    
    info!("🎵 Starting Event-Driven Foreign Instruments Bridge (rusb)");
    info!("=========================================================");
    info!("Features:");
    info!("  ✅ USB device support (Maschine Jam, etc.)");
    info!("  ✅ Event-driven architecture (no polling)");
    info!("  ✅ Multi-threaded input/output");
    info!("  ✅ Real-time USB data reading");
    info!("");

    let mut app = EventDrivenRusbApp::new().await?;
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

    info!("🎵 Foreign Instruments Bridge (rusb) stopped");
    Ok(())
}

pub async fn test_rusb_devices() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("🔍 Testing Rusb Device Detection");
    info!("================================");

    let (rusb_event_sender, _rusb_event_receiver) = mpsc::unbounded_channel::<RusbHidEvent>();
    let rusb_manager = RusbHidManager::new(rusb_event_sender)?;
    rusb_manager.scan_initial_devices()?;
    let devices = rusb_manager.get_devices();
    info!("Found {} USB devices:", devices.len());
    for device in &devices {
        info!("  🎹 {} ({:04x}:{:04x}) - {:?}", 
              device.name, device.vendor_id, device.product_id, device.state);
    }
    if devices.is_empty() {
        warn!("No USB devices found!");
    } else {
        info!("✅ USB device detection working!");
    }
    info!("");
    info!("Now you can test the event-driven architecture:");
    info!("1. Connect/disconnect your Maschine Jam");
    info!("2. Watch for USB events in the logs");
    info!("3. See real USB data from button presses");
    Ok(())
} 