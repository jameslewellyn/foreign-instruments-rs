use rusb::{Context, DeviceHandle, UsbContext, Direction, TransferType};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use log::{info, warn, error, debug};
use tokio::sync::mpsc;
use crate::types::foreign_instruments_types::{DeviceState, ManagedDevice};

// HID event types (same as hid_devices.rs)
#[derive(Debug, Clone)]
pub enum RusbHidEvent {
    DeviceConnected { vendor_id: u16, product_id: u16 },
    DeviceDisconnected { vendor_id: u16, product_id: u16 },
    InputReport { vendor_id: u16, product_id: u16, data: Vec<u8> },
    Error { vendor_id: u16, product_id: u16, error: String },
}

struct RusbDeviceReader {
    handle: DeviceHandle<Context>,
    interface: u8,
    endpoint: u8,
    stop_flag: Arc<Mutex<bool>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

// Rusb-based HID device manager
pub struct RusbHidManager {
    context: Context,
    devices: Arc<Mutex<Vec<ManagedDevice>>>,
    event_sender: mpsc::UnboundedSender<RusbHidEvent>,
    running: Arc<Mutex<bool>>,
    readers: Arc<Mutex<HashMap<(u16, u16), RusbDeviceReader>>>,
}

impl RusbHidManager {
    pub fn new(event_sender: mpsc::UnboundedSender<RusbHidEvent>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let context = Context::new()?;
        Ok(Self {
            context,
            devices: Arc::new(Mutex::new(Vec::new())),
            event_sender,
            running: Arc::new(Mutex::new(true)),
            readers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    // Check if a device is a Native Instruments device
    pub fn is_native_instruments_device(vendor_id: u16, _product_id: u16) -> bool {
        vendor_id == 0x17cc // Native Instruments vendor ID
    }

    // Check if a device is interesting (Native Instruments or other MIDI devices)
    pub fn is_interesting_device(vendor_id: u16, product_id: u16) -> bool {
        Self::is_native_instruments_device(vendor_id, product_id)
            || vendor_id == 0x0763 // M-Audio
            || vendor_id == 0x1235 // Focusrite
            || vendor_id == 0x1bcf // Arturia
    }

    // Find the first interrupt IN endpoint for a device
    fn find_interrupt_endpoint(&self, device: &rusb::Device<Context>, interface: u8) -> Option<u8> {
        if let Ok(config) = device.active_config_descriptor() {
            for interface_desc in config.interfaces() {
                if interface_desc.number() == interface {
                    for desc in interface_desc.descriptors() {
                        for ep in desc.endpoint_descriptors() {
                            if ep.direction() == Direction::In && ep.transfer_type() == TransferType::Interrupt {
                                return Some(ep.address());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // Scan for initial USB devices
    pub fn scan_initial_devices(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Scanning for initial USB devices...");
        
        for device in self.context.devices()?.iter() {
            if let Ok(desc) = device.device_descriptor() {
                let vendor_id = desc.vendor_id();
                let product_id = desc.product_id();
                
                if Self::is_interesting_device(vendor_id, product_id) {
                    info!("Found interesting USB device: {:04x}:{:04x}", vendor_id, product_id);
                    
                    // Try to open and read from the device
                    match self.open_and_read_device(vendor_id, product_id) {
                        Ok(_) => {
                            let managed_device = ManagedDevice {
                                vendor_id,
                                product_id,
                                name: format!("Device {:04x}:{:04x}", vendor_id, product_id),
                                state: DeviceState::Active,
                            };
                            
                            // Add to device list
                            {
                                let mut devices = self.devices.lock().unwrap();
                                devices.push(managed_device);
                            }
                            
                            // Send device connected event
                            let _ = self.event_sender.send(RusbHidEvent::DeviceConnected {
                                vendor_id,
                                product_id,
                            });
                        }
                        Err(e) => {
                            warn!("Failed to open USB device {:04x}:{:04x}: {}", vendor_id, product_id, e);
                        }
                    }
                }
            }
        }
        
        info!("USB device scan complete");
        Ok(())
    }

    fn open_and_read_device(&self, vendor_id: u16, product_id: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let device = self.context.open_device_with_vid_pid(vendor_id, product_id)
            .ok_or_else(|| format!("Device {:04x}:{:04x} not found", vendor_id, product_id))?;
        
        let interface = 0; // Most HID devices use interface 0
        
        // Find the interrupt endpoint
        let endpoint = self.find_interrupt_endpoint(&device.device(), interface)
            .ok_or_else(|| format!("No interrupt endpoint found for {:04x}:{:04x}", vendor_id, product_id))?;
        
        info!("Opening device {:04x}:{:04x} on interface {} endpoint 0x{:02x}", 
              vendor_id, product_id, interface, endpoint);
        
        // Detach kernel driver if necessary
        if device.kernel_driver_active(interface).unwrap_or(false) {
            device.detach_kernel_driver(interface)?;
        }
        
        // Claim the interface
        device.claim_interface(interface)?;
        
        // Start reading thread
        self.start_reader_thread(vendor_id, product_id, device, interface, endpoint);
        
        Ok(())
    }

    fn start_reader_thread(&self, vendor_id: u16, product_id: u16, device: DeviceHandle<Context>, 
                          interface: u8, endpoint: u8) {
        let event_sender = self.event_sender.clone();
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let key = (vendor_id, product_id);
        
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 64];
            let timeout = Duration::from_millis(100);
            
            loop {
                if *stop_flag_clone.lock().unwrap() {
                    break;
                }
                
                match device.read_interrupt(endpoint, &mut buf, timeout) {
                    Ok(len) if len > 0 => {
                        let data = buf[..len].to_vec();
                        debug!("USB data from {:04x}:{:04x}: {:02x?}", vendor_id, product_id, data);
                        let _ = event_sender.send(RusbHidEvent::InputReport {
                            vendor_id,
                            product_id,
                            data,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        if e == rusb::Error::Timeout {
                            continue;
                        } else {
                            error!("USB read error for {:04x}:{:04x}: {}", vendor_id, product_id, e);
                            let _ = event_sender.send(RusbHidEvent::DeviceDisconnected { vendor_id, product_id });
                            break;
                        }
                    }
                }
            }
        });
        
        let mut readers = self.readers.lock().unwrap();
        readers.insert(key, RusbDeviceReader {
            handle,
            interface,
            endpoint,
            stop_flag,
            thread_handle: Some(handle),
        });
    }

    fn stop_reader_thread(&self, vendor_id: u16, product_id: u16) {
        let key = (vendor_id, product_id);
        let mut readers = self.readers.lock().unwrap();
        if let Some(reader) = readers.remove(&key) {
            *reader.stop_flag.lock().unwrap() = true;
            if let Some(handle) = reader.thread_handle {
                let _ = handle.join();
            }
        }
    }

    // Start monitoring USB devices (event-driven)
    pub fn start_monitoring(&self) {
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();
        let devices = self.devices.clone();
        let readers = self.readers.clone();
        let context = self.context.clone();
        
        thread::spawn(move || {
            info!("USB device monitoring thread started");
            let mut known_devices = HashMap::new();
            
            while *running.lock().unwrap() {
                thread::sleep(Duration::from_millis(1000));
                let mut current_devices = HashMap::new();
                
                for device in context.devices().unwrap().iter() {
                    if let Ok(desc) = device.device_descriptor() {
                        let vendor_id = desc.vendor_id();
                        let product_id = desc.product_id();
                        
                        if RusbHidManager::is_interesting_device(vendor_id, product_id) {
                            current_devices.insert((vendor_id, product_id), device.address());
                            
                            if !known_devices.contains_key(&(vendor_id, product_id)) {
                                info!("Hotplug: New USB device {:04x}:{:04x}", vendor_id, product_id);
                                let _ = event_sender.send(RusbHidEvent::DeviceConnected { vendor_id, product_id });
                            }
                        }
                    }
                }
                
                // Check for disconnected devices
                let disconnected: Vec<_> = known_devices.keys()
                    .filter(|&&(vid, pid)| !current_devices.contains_key(&(vid, pid)))
                    .cloned()
                    .collect();
                
                for (vendor_id, product_id) in disconnected {
                    info!("Hotplug: USB device disconnected {:04x}:{:04x}", vendor_id, product_id);
                    let _ = event_sender.send(RusbHidEvent::DeviceDisconnected { vendor_id, product_id });
                    known_devices.remove(&(vendor_id, product_id));
                }
                
                known_devices = current_devices;
            }
        });
    }

    pub fn stop_monitoring(&self) {
        *self.running.lock().unwrap() = false;
    }

    pub fn get_devices(&self) -> Vec<ManagedDevice> {
        self.devices.lock().unwrap().clone()
    }

    pub fn update_device_state(&self, vendor_id: u16, product_id: u16, state: DeviceState) {
        let mut devices = self.devices.lock().unwrap();
        if let Some(dev) = devices.iter_mut().find(|d| d.vendor_id == vendor_id && d.product_id == product_id) {
            dev.state = state;
        }
    }
}

// Basic event handler that logs events
pub struct BasicRusbHidEventHandler {}

impl BasicRusbHidEventHandler {
    pub async fn handle_event(&mut self, event: RusbHidEvent) {
        match event {
            RusbHidEvent::DeviceConnected { vendor_id, product_id } => {
                info!("🎹 USB Device Connected: {:04x}:{:04x}", vendor_id, product_id);
            }
            RusbHidEvent::DeviceDisconnected { vendor_id, product_id } => {
                info!("🔌 USB Device Disconnected: {:04x}:{:04x}", vendor_id, product_id);
            }
            RusbHidEvent::InputReport { vendor_id, product_id, data } => {
                info!("📥 USB Input from {:04x}:{:04x}: {:02x?}", vendor_id, product_id, data);
            }
            RusbHidEvent::Error { vendor_id, product_id, error } => {
                error!("❌ USB Error for {:04x}:{:04x}: {}", vendor_id, product_id, error);
            }
        }
    }
}

// Process rusb HID events
pub async fn process_rusb_hid_events(mut handler: BasicRusbHidEventHandler, mut event_receiver: mpsc::UnboundedReceiver<RusbHidEvent>) {
    info!("Starting rusb HID event processing...");
    
    while let Some(event) = event_receiver.recv().await {
        handler.handle_event(event).await;
    }
    
    info!("Rusb HID event processing stopped");
} 