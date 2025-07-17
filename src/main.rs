mod midi;
mod types;

use rusb::{Context, UsbContext, Device, Hotplug, HotplugBuilder};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex};
use std::thread;
use std::time::Duration;
use log::{info, warn, error};
use once_cell::sync::Lazy;
use crate::types::foreign_instruments_types::{ManagedDevice, DeviceState};
use crate::midi::{MidiTranslator, UsbMessage, MidiMapping};

const NATIVE_INSTRUMENTS_VID: u16 = 0x17CC;
const COMMON_MIDI_VIDS: [u16; 3] = [0x0763, 0x1235, 0x1BCF];

static DEVICE_REGISTRY: Lazy<Mutex<Vec<ManagedDevice>>> = Lazy::new(|| Mutex::new(Vec::new()));

fn is_interesting_device(vid: u16) -> bool {
    vid == NATIVE_INSTRUMENTS_VID || COMMON_MIDI_VIDS.contains(&vid)
}

fn update_device_state(vendor_id: u16, product_id: u16, new_state: DeviceState) {
    let mut registry = DEVICE_REGISTRY.lock().unwrap();
    let id = (vendor_id, product_id);
    if let Some(dev) = registry.iter_mut().find(|d| d.vendor_id == id.0 && d.product_id == id.1) {
        let old_state = dev.state.clone();
        dev.state = new_state.clone();
        info!("Device state transition: {:?} -> {:?} for {:?}", old_state, new_state, dev);
    } else {
        let dev = ManagedDevice {
            name: format!("Device {:04x}:{:04x}", id.0, id.1),
            vendor_id: id.0,
            product_id: id.1,
            state: new_state.clone(),
        };
        info!("New device with state {:?}: {:?}", new_state, dev);
        registry.push(dev);
    }
}

fn handle_device_error(vendor_id: u16, product_id: u16, error_msg: String) {
    error!("Device error for {:04x}:{:04x}: {}", vendor_id, product_id, error_msg);
    update_device_state(vendor_id, product_id, DeviceState::Error(error_msg));
}

fn simulate_device_input(midi_translator: &mut MidiTranslator, vendor_id: u16, product_id: u16) {
    // Get default mapping for this device
    let mapping = midi_translator.translator.get_default_mapping(vendor_id, product_id);
    
    // Simulate some USB messages for testing
    let test_messages = vec![
        UsbMessage::Button { button_id: 0, pressed: true },
        UsbMessage::Button { button_id: 0, pressed: false },
        UsbMessage::Pad { pad_id: 0, velocity: 100, pressed: true },
        UsbMessage::Pad { pad_id: 0, velocity: 0, pressed: false },
        UsbMessage::Knob { knob_id: 0, value: 64 },
        UsbMessage::Knob { knob_id: 0, value: 127 },
    ];
    
    for usb_msg in test_messages {
        if let Err(e) = midi_translator.translate_and_send(&usb_msg, &mapping) {
            error!("Failed to translate and send USB message: {}", e);
        }
        thread::sleep(Duration::from_millis(100)); // Small delay between messages
    }
}

struct PrintHotplug {
    midi_translator: Arc<Mutex<MidiTranslator>>,
}

impl<T: UsbContext> Hotplug<T> for PrintHotplug {
    fn device_arrived(&mut self, device: Device<T>) {
        if let Ok(desc) = device.device_descriptor() {
            if is_interesting_device(desc.vendor_id()) {
                let vendor_id = desc.vendor_id();
                let product_id = desc.product_id();
                
                // Attempt to initialize the device
                match initialize_device(&device, &desc) {
                    Ok(_) => {
                        update_device_state(vendor_id, product_id, DeviceState::Active);
                        info!(
                            "[HOTPLUG] Connected: Bus {:03} Device {:03} ID {:04x}:{:04x}",
                            device.bus_number(),
                            device.address(),
                            vendor_id,
                            product_id
                        );
                        
                        // Simulate device input for testing
                        if let Ok(mut midi) = self.midi_translator.lock() {
                            simulate_device_input(&mut midi, vendor_id, product_id);
                        }
                    }
                    Err(e) => {
                        handle_device_error(vendor_id, product_id, format!("Initialization failed: {}", e));
                    }
                }
            }
        } else {
            error!("Failed to get device descriptor for device on bus {:03} device {:03}",
                   device.bus_number(), device.address());
        }
    }
    
    fn device_left(&mut self, device: Device<T>) {
        if let Ok(desc) = device.device_descriptor() {
            if is_interesting_device(desc.vendor_id()) {
                let vendor_id = desc.vendor_id();
                let product_id = desc.product_id();
                
                update_device_state(vendor_id, product_id, DeviceState::Disconnected);
                info!(
                    "[HOTPLUG] Disconnected: Bus {:03} Device {:03} ID {:04x}:{:04x}",
                    device.bus_number(),
                    device.address(),
                    vendor_id,
                    product_id
                );
            }
        } else {
            warn!("Failed to get device descriptor for disconnected device on bus {:03} device {:03}",
                  device.bus_number(), device.address());
        }
    }
}

fn initialize_device<T: UsbContext>(device: &Device<T>, desc: &rusb::DeviceDescriptor) -> Result<(), String> {
    // Simulate device initialization with potential errors
    let vendor_id = desc.vendor_id();
    let product_id = desc.product_id();
    
    // Check if device is supported
    if !is_interesting_device(vendor_id) {
        return Err("Device not in supported vendor list".to_string());
    }
    
    // Simulate initialization delay and potential failure
    if vendor_id == NATIVE_INSTRUMENTS_VID && product_id == 0x1500 {
        // Simulate a specific device that might fail initialization
        if rand::random::<f32>() < 0.1 {
            return Err("Random initialization failure for testing".to_string());
        }
    }
    
    info!("Device initialization successful for {:04x}:{:04x}", vendor_id, product_id);
    Ok(())
}

fn print_device_registry() {
    let registry = DEVICE_REGISTRY.lock().unwrap();
    if registry.is_empty() {
        info!("No devices currently tracked");
    } else {
        info!("Current device registry:");
        for (i, device) in registry.iter().enumerate() {
            info!("  {}. {:?}", i + 1, device);
        }
    }
}

fn main() {
    env_logger::init();
    let context = Context::new().expect("Failed to create USB context");
    if !rusb::has_hotplug() {
        error!("Hotplug support is not available on this platform.");
        return;
    }

    // Set up virtual MIDI port
    let midi_translator = match MidiTranslator::new("Foreign Instruments Virtual Port") {
        Ok(m) => Arc::new(Mutex::new(m)),
        Err(e) => {
            error!("Failed to create virtual MIDI port: {}", e);
            return;
        }
    };
    
    // Send initial test message
    if let Ok(mut midi) = midi_translator.lock() {
        midi.send_test_message();
    }

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Create hotplug handler with MIDI translator
    let hotplug_handler = PrintHotplug {
        midi_translator: midi_translator.clone(),
    };

    // Register hotplug callback using modern HotplugBuilder
    let _registration: rusb::Registration<rusb::Context> = HotplugBuilder::new()
        .register(&context, Box::new(hotplug_handler))
        .expect("Failed to register hotplug callback");

    info!("Listening for hotplug events for Native Instruments and common MIDI devices. Press Ctrl+C to exit.");

    // Main event loop
    let mut tick_count = 0;
    while running_clone.load(Ordering::SeqCst) {
        if let Err(e) = context.handle_events(None) {
            error!("Error handling USB events: {}", e);
        }
        
        // Print device registry every 10 seconds
        tick_count += 1;
        if tick_count % 100 == 0 { // 100 * 100ms = 10 seconds
            print_device_registry();
        }
        
        thread::sleep(Duration::from_millis(100));
    }
} 