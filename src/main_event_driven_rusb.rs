use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::init();
    
    info!("🎵 Foreign Instruments Bridge - Event-Driven Version (rusb)");
    info!("==========================================================");
    info!("This version features:");
    info!("  ✅ USB device support for Native Instruments hardware");
    info!("  ✅ Event-driven architecture (no polling loops)");
    info!("  ✅ Multi-threaded input/output for maximum efficiency");
    info!("  ✅ Real-time USB data reading from hardware");
    info!("");

    // Test USB device detection first
    info!("Testing USB device detection...");
    foreigninstruments::event_driven_main_rusb::test_rusb_devices().await?;
    
    info!("");
    info!("Press Enter to start the event-driven bridge (rusb)...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    // Start the event-driven application
    foreigninstruments::event_driven_main_rusb::run_event_driven_rusb_app().await?;

    Ok(())
} 