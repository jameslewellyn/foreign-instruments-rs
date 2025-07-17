use rusb::{Context, UsbContext, DeviceHandle, DeviceDescriptor};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use log::{info, error};
mod midi;
mod midi_mapping;
use midi::UsbToMidiTranslator;

const POLL_INTERVAL_MS: u64 = 10;
const NATIVE_INSTRUMENTS_VID: u16 = 0x17cc;
const COMMON_MIDI_VIDS: [u16; 3] = [0x0763, 0x1235, 0x1bcf];

fn is_interesting_device(vid: u16) -> bool {
    vid == NATIVE_INSTRUMENTS_VID || COMMON_MIDI_VIDS.contains(&vid)
}

fn poll_usb_devices<T: UsbContext>(context: &T, mapping_translator: &impl UsbToMidiTranslator, midi: &mut midi::MidiTranslator) {
    for device in context.devices().unwrap().iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        if !is_interesting_device(desc.vendor_id()) {
            continue;
        }
        let mut handle = match device.open() {
            Ok(h) => h,
            Err(e) => {
                error!("Failed to open device {:04x}:{:04x}: {}", desc.vendor_id(), desc.product_id(), e);
                continue;
            }
        };
        // Try to claim all interfaces (for simplicity)
        for iface in 0..desc.num_configurations() {
            let _ = handle.claim_interface(iface);
        }
        // For demo: try to read from endpoint 0x81 (common for interrupt in)
        let mut buf = [0u8; 64];
        match handle.read_interrupt(0x81, &mut buf, Duration::from_millis(POLL_INTERVAL_MS)) {
            Ok(len) if len > 0 => {
                let usb_data = &buf[..len];
                let midi_msgs = mapping_translator.translate_usb_to_midi(usb_data, Some(desc.vendor_id()), Some(desc.product_id()));
                for msg in midi_msgs {
                    info!("USB data {:?} mapped to MIDI message {:?}", usb_data, msg);
                    midi.send_midi_message(&msg);
                }
            }
            _ => {}
        }
    }
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
    let context = Context::new().expect("Failed to create USB context");
    if !rusb::has_hotplug() {
        error!("Hotplug support is not available on this platform.");
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

    info!("Listening for hotplug events for Native Instruments and common MIDI devices. Press Ctrl+C to exit.");

    // Main event loop
    let mut tick_count = 0;
    while running_clone.load(Ordering::SeqCst) {
        // Poll real USB devices
        poll_usb_devices(&context, &mapping_translator, &mut midi);

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
        tick_count += 1;
        thread::sleep(Duration::from_millis(100));
    }
} 