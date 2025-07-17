use rusb::{Context, UsbContext, DeviceHandle, DeviceDescriptor};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use log::{info, error, warn};
mod midi;
mod midi_mapping;
mod device_registry;
mod hotplug_handler;
mod types;
mod test_detection;
use crate::types::foreign_instruments_types::DeviceState;
use midi::UsbToMidiTranslator;
use device_registry::DeviceRegistry;
use hotplug_handler::HotplugHandler;

const POLL_INTERVAL_MS: u64 = 10;
const NATIVE_INSTRUMENTS_VID: u16 = 0x17cc;
const COMMON_MIDI_VIDS: [u16; 3] = [0x0763, 0x1235, 0x1bcf];

fn is_interesting_device(vid: u16) -> bool {
    vid == NATIVE_INSTRUMENTS_VID || COMMON_MIDI_VIDS.contains(&vid)
}

fn poll_usb_devices(registry: &DeviceRegistry, mapping_translator: &impl UsbToMidiTranslator, midi: &mut midi::MidiTranslator) {
    let active_devices = registry.get_active_devices();
    for device in active_devices {
        if let Some(usb_device) = find_device_by_vid_pid(device.vendor_id, device.product_id) {
            let desc = match usb_device.device_descriptor() {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to get device descriptor for {:04x}:{:04x}: {}", device.vendor_id, device.product_id, e);
                    registry.update_device_state(device.vendor_id, device.product_id, DeviceState::Error(format!("Descriptor error: {}", e)));
                    continue;
                }
            };
            let mut handle = match usb_device.open() {
                Ok(h) => h,
                Err(e) => {
                    error!("Failed to open device {:04x}:{:04x}: {}", device.vendor_id, device.product_id, e);
                    registry.update_device_state(device.vendor_id, device.product_id, DeviceState::Error(format!("Open error: {}", e)));
                    continue;
                }
            };
            if let Err(e) = handle.claim_interface(0) {
                warn!("Failed to claim interface 0 for {:04x}:{:04x}: {}", device.vendor_id, device.product_id, e);
            }
            let mut buf = [0u8; 64];
            match handle.read_interrupt(0x81, &mut buf, std::time::Duration::from_millis(POLL_INTERVAL_MS)) {
                Ok(len) if len > 0 => {
                    let usb_data = &buf[..len];
                    let midi_msgs = mapping_translator.translate_usb_to_midi(usb_data, Some(device.vendor_id), Some(device.product_id));
                    for msg in midi_msgs {
                        info!("USB data {:?} from {} mapped to MIDI message {:?}", usb_data, device.name, msg);
                        midi.send_midi_message(&msg);
                    }
                }
                Ok(_) => {
                    registry.update_device_state(device.vendor_id, device.product_id, DeviceState::Active);
                }
                Err(e) => {
                    warn!("Failed to read from device {:04x}:{:04x}: {}", device.vendor_id, device.product_id, e);
                    registry.update_device_state(device.vendor_id, device.product_id, DeviceState::Error(format!("Read error: {}", e)));
                }
            }
        } else {
            registry.update_device_state(device.vendor_id, device.product_id, DeviceState::Disconnected);
        }
    }
}

fn find_device_by_vid_pid(vid: u16, pid: u16) -> Option<rusb::Device<Context>> {
    let context = Context::new().ok()?;
    for device in context.devices().ok()?.iter() {
        if let Ok(desc) = device.device_descriptor() {
            if desc.vendor_id() == vid && desc.product_id() == pid {
                return Some(device);
            }
        }
    }
    None
}

fn simulate_usb_messages() -> Vec<(Vec<u8>, Option<u16>, Option<u16>)> {
    vec![
        (vec![1, 15, 99, 42], Some(0x17cc), Some(0x1500)), // Should match first mapping (range + exact)
        (vec![5, 123, 7], None, None), // Should match second mapping (wildcard)
        (vec![8, 12, 30], None, None), // Should match third mapping (range)
        (vec![1, 2, 3], None, None), // Should not match any mapping
    ]
}

fn main() {
    env_logger::init();
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    
    // Run device detection test first
    test_detection::test_device_detection();
    println!();
    println!("🎵 HOTPLUG TEST READY! 🎵");
    println!("==========================");
    println!("Your Maschine Jam is detected and ready for testing.");
    println!("The application will now monitor for hotplug events.");
    println!("");
    println!("📋 Test Instructions:");
    println!("1. Disconnect your Maschine Jam USB cable");
    println!("2. Wait 2-3 seconds");
    println!("3. Reconnect your Maschine Jam USB cable");
    println!("4. Watch for hotplug events in the logs below");
    println!("");
    println!("Press Enter to start monitoring...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    // Create device registry
    let registry = Arc::new(DeviceRegistry::new());
    // Create hotplug handler
    let mut hotplug_handler = match HotplugHandler::new(Arc::clone(&registry)) {
        Ok(handler) => handler,
        Err(e) => {
            error!("Failed to create hotplug handler: {}", e);
            return;
        }
    };
    // Register hotplug callbacks
    if let Err(e) = hotplug_handler.register_hotplug_callbacks() {
        error!("Failed to register hotplug callbacks: {}", e);
        return;
    }
    // Scan for initial devices
    if let Err(e) = hotplug_handler.scan_initial_devices() {
        error!("Failed to scan initial devices: {}", e);
        return;
    }
    // Set up virtual MIDI port
    let mut midi = match midi::MidiTranslator::new("Foreign Instruments Virtual Port") {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to create virtual MIDI port: {}", e);
            return;
        }
    };
    midi.send_test_message();
    // Load mapping config
    let mapping_translator = match midi::MappingUsbToMidi::from_config_file("midi_mapping.toml") {
        Ok(mt) => mt,
        Err(e) => {
            error!("Failed to load MIDI mapping config: {}", e);
            return;
        }
    };
    info!("Loaded {} MIDI mapping entries", mapping_translator.mappings.len());
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    info!("Foreign Instruments Bridge started successfully!");
    info!("Listening for hotplug events and managing device lifecycle.");
    info!("Press Ctrl+C to exit.");
    // Main event loop
    let mut tick_count = 0;
    while running_clone.load(Ordering::SeqCst) {
        // Poll device status (check for disconnected devices)
        if let Err(e) = hotplug_handler.poll_devices() {
            error!("Failed to poll devices: {}", e);
        }
        // Poll active USB devices for data
        poll_usb_devices(&registry, &mapping_translator, &mut midi);
        // Simulate USB messages every 2 seconds (for fallback/testing)
        if tick_count % 20 == 0 {
            for (usb_data, vendor_id, product_id) in simulate_usb_messages() {
                let midi_msgs = mapping_translator.translate_usb_to_midi(&usb_data, vendor_id, product_id);
                if midi_msgs.is_empty() {
                    info!("No mapping for simulated USB data: {:?}", usb_data);
                } else {
                    for msg in midi_msgs {
                        info!("Simulated USB data {:?} mapped to MIDI message {:?}", usb_data, msg);
                        midi.send_midi_message(&msg);
                    }
                }
            }
        }
        // Print device status every 10 seconds
        if tick_count % 100 == 0 {
            registry.print_status();
        }
        tick_count += 1;
        thread::sleep(std::time::Duration::from_millis(100));
    }
} 