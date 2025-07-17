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

// Remove 'handle' from the struct
struct RusbDeviceReader {
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

    // Find the first interrupt IN endpoint for a device, searching all interfaces
    fn find_interrupt_endpoint(&self, device: &rusb::Device<Context>) -> Option<(u8, u8)> {
        if let Ok(config) = device.active_config_descriptor() {
            for interface in config.interfaces() {
                let iface_num = interface.number();
                for desc in interface.descriptors() {
                    for ep in desc.endpoint_descriptors() {
                        log::info!("Examining interface {} endpoint 0x{:02x} (dir={:?}, type={:?})", iface_num, ep.address(), ep.direction(), ep.transfer_type());
                        if ep.direction() == Direction::In && ep.transfer_type() == TransferType::Interrupt {
                            log::info!("Selected interface {} endpoint 0x{:02x} (interrupt IN)", iface_num, ep.address());
                            return Some((iface_num, ep.address()));
                        }
                    }
                }
            }
        }
        None
    }

    // Scan for initial USB devices
    pub fn scan_initial_devices(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("=== Starting USB device scan ===");
        
        let devices_result = self.context.devices();
        log::info!("Context.devices() result: {:?}", devices_result.as_ref().map(|_| "Ok").unwrap_or("Err"));
        let devices = devices_result?;
        log::info!("Found {} total USB devices", devices.len());
        
        for (i, device) in devices.iter().enumerate() {
            log::info!("Examining device #{}", i);
            let desc_result = device.device_descriptor();
            log::info!("Device #{} descriptor result: {:?}", i, desc_result);
            if let Ok(desc) = desc_result {
                let vendor_id = desc.vendor_id();
                let product_id = desc.product_id();
                log::info!("Device #{}: {:04x}:{:04x}", i, vendor_id, product_id);
                
                if Self::is_interesting_device(vendor_id, product_id) {
                    log::info!("Device {:04x}:{:04x} is interesting - attempting to open", vendor_id, product_id);
                    
                    // Try to open and read from the device
                    match self.open_and_read_device(vendor_id, product_id) {
                        Ok(_) => {
                            log::info!("Successfully opened device {:04x}:{:04x}", vendor_id, product_id);
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
                            log::warn!("Failed to open USB device {:04x}:{:04x}: {}", vendor_id, product_id, e);
                        }
                    }
                } else {
                    log::info!("Device {:04x}:{:04x} is not interesting", vendor_id, product_id);
                }
            } else {
                log::warn!("Failed to get descriptor for device #{}", i);
            }
        }
        
        log::info!("=== USB device scan complete ===");
        Ok(())
    }

    fn open_and_read_device(&self, vendor_id: u16, product_id: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("=== Attempting to open device {:04x}:{:04x} ===", vendor_id, product_id);
        
        log::info!("Calling context.open_device_with_vid_pid({:04x}, {:04x})", vendor_id, product_id);
        let device = self.context.open_device_with_vid_pid(vendor_id, product_id);
        log::info!("open_device_with_vid_pid result: {:?}", device);
        let device = device.ok_or_else(|| format!("Device {:04x}:{:04x} not found", vendor_id, product_id))?;
        log::info!("Device opened successfully");

        log::info!("Getting device descriptor");
        let device_desc = device.device();
        log::info!("Device descriptor obtained");

        // Find the interrupt endpoint and interface
        log::info!("Searching for interrupt IN endpoint");
        let endpoint_result = self.find_interrupt_endpoint(&device_desc);
        log::info!("find_interrupt_endpoint result: {:?}", endpoint_result);
        let (interface, endpoint) = endpoint_result
            .ok_or_else(|| format!("No interrupt IN endpoint found for {:04x}:{:04x}", vendor_id, product_id))?;
        log::info!("Using interface {} endpoint 0x{:02x}", interface, endpoint);

        // Detach kernel driver if necessary
        log::info!("Checking if kernel driver is active on interface {}", interface);
        let kernel_driver_active = device.kernel_driver_active(interface);
        log::info!("kernel_driver_active result: {:?}", kernel_driver_active);
        if kernel_driver_active.unwrap_or(false) {
            log::info!("Detaching kernel driver from interface {}", interface);
            let detach_result = device.detach_kernel_driver(interface);
            log::info!("detach_kernel_driver result: {:?}", detach_result);
            detach_result?;
        } else {
            log::info!("No kernel driver to detach on interface {}", interface);
        }
        
        log::info!("Claiming interface {}", interface);
        let claim_result = device.claim_interface(interface);
        log::info!("claim_interface result: {:?}", claim_result);
        claim_result?;
        log::info!("Interface claimed successfully");

        // Start reading thread
        log::info!("Starting reader thread for device {:04x}:{:04x}", vendor_id, product_id);
        self.start_reader_thread(vendor_id, product_id, device, interface, endpoint);
        log::info!("Reader thread started successfully");
        
        log::info!("=== Device {:04x}:{:04x} setup complete ===", vendor_id, product_id);
        Ok(())
    }

    fn start_reader_thread(&self, vendor_id: u16, product_id: u16, device: DeviceHandle<Context>, 
                          interface: u8, endpoint: u8) {
        log::info!("=== Starting reader thread for {:04x}:{:04x} ===", vendor_id, product_id);
        let event_sender = self.event_sender.clone();
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let key = (vendor_id, product_id);
        
        let thread_handle = thread::spawn(move || {
            log::info!("Reader thread started for {:04x}:{:04x}", vendor_id, product_id);
            let mut buf = [0u8; 64];
            let timeout = Duration::from_millis(100);
            let mut read_count = 0;
            
            loop {
                if *stop_flag_clone.lock().unwrap() {
                    log::info!("Reader thread stopping for {:04x}:{:04x}", vendor_id, product_id);
                    break;
                }
                
                read_count += 1;
                log::debug!("Attempting read #{} for {:04x}:{:04x}", read_count, vendor_id, product_id);
                match device.read_interrupt(endpoint, &mut buf, timeout) {
                    Ok(len) if len > 0 => {
                        let data = buf[..len].to_vec();
                        log::info!("📥 USB data from {:04x}:{:04x} (read #{}): {:02x?}", vendor_id, product_id, read_count, data);
                        let send_result = event_sender.send(RusbHidEvent::InputReport {
                            vendor_id,
                            product_id,
                            data,
                        });
                        log::info!("📤 Sending InputReport event for {:04x}:{:04x}, result: {:?}", vendor_id, product_id, send_result);
                    }
                    Ok(_) => {
                        log::debug!("Read #{} for {:04x}:{:04x}: no data", read_count, vendor_id, product_id);
                    }
                    Err(e) => {
                        if e == rusb::Error::Timeout {
                            log::debug!("Read #{} for {:04x}:{:04x}: timeout", read_count, vendor_id, product_id);
                            continue;
                        } else {
                            log::error!("USB read error for {:04x}:{:04x} (read #{}): {}", vendor_id, product_id, read_count, e);
                            let send_result = event_sender.send(RusbHidEvent::DeviceDisconnected { vendor_id, product_id });
                            log::debug!("Disconnect event send result: {:?}", send_result);
                            break;
                        }
                    }
                }
            }
            log::info!("Reader thread ended for {:04x}:{:04x}", vendor_id, product_id);
        });
        
        log::info!("Storing reader thread handle for {:04x}:{:04x}", vendor_id, product_id);
        let mut readers = self.readers.lock().unwrap();
        readers.insert(key, RusbDeviceReader {
            interface,
            endpoint,
            stop_flag,
            thread_handle: Some(thread_handle),
        });
        log::info!("Reader thread handle stored successfully");
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