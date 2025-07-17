use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::init();
    
    info!("🎵 Foreign Instruments Bridge - Event-Driven Version");
    info!("===================================================");
    info!("This version features:");
    info!("  ✅ HID device support for Native Instruments hardware");
    info!("  ✅ Event-driven architecture (no polling loops)");
    info!("  ✅ Multi-threaded input/output for maximum efficiency");
    info!("  ✅ Real-time MIDI translation from HID events");
    info!("");

    // Test HID device detection first
    info!("Testing HID device detection...");
    foreigninstruments::event_driven_main::test_hid_devices().await?;
    
    info!("");
    info!("Press Enter to start the event-driven bridge...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    // Start the event-driven application
    foreigninstruments::event_driven_main::run_event_driven_app().await?;

    Ok(())
} 