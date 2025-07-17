use hidapi::{HidApi, HidDevice, HidError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use log::{info, warn, error, debug};
use tokio::sync::mpsc;
use crate::types::foreign_instruments_types::{DeviceState, ManagedDevice};

// HID event types
#[derive(Debug, Clone)]
pub enum HidEvent {
    DeviceConnected { vendor_id: u16, product_id: u16 },
    DeviceDisconnected { vendor_id: u16, product_id: u16 },
    InputReport { vendor_id: u16, product_id: u16, data: Vec<u8> },
    Error { vendor_id: u16, product_id: u16, error: String },
}

struct DeviceReaderHandle {
    stop_flag: Arc<Mutex<bool>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

// HID device manager
pub struct HidDeviceManager {
    api: Arc<Mutex<HidApi>>,
    devices: Arc<Mutex<Vec<ManagedDevice>>>,
    event_sender: mpsc::UnboundedSender<HidEvent>,
    running: Arc<Mutex<bool>>,
    readers: Arc<Mutex<HashMap<(u16, u16), DeviceReaderHandle>>>,
}

impl HidDeviceManager {
    pub fn new(event_sender: mpsc::UnboundedSender<HidEvent>) -> Result<Self, HidError> {
        let api = HidApi::new()?;
        Ok(Self {
            api: Arc::new(Mutex::new(api)),
            devices: Arc::new(Mutex::new(Vec::new())),
            event_sender,
            running: Arc::new(Mutex::new(true)),
            readers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn event_sender(&self) -> mpsc::UnboundedSender<HidEvent> {
        self.event_sender.clone()
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

    // Scan for initial HID devices
    pub fn scan_initial_devices(&self) -> Result<(), HidError> {
        info!("Scanning for initial HID devices...");
        let api = self.api.lock().unwrap();
        for device_info in api.device_list() {
            let vendor_id = device_info.vendor_id();
            let product_id = device_info.product_id();
            
            if Self::is_interesting_device(vendor_id, product_id) {
                info!("Found interesting HID device: {:04x}:{:04x} - {}", 
                      vendor_id, product_id, device_info.product_string().unwrap_or("Unknown"));
                
                // Try to open the device
                match device_info.open_device(&api) {
                    Ok(device) => {
                        let managed_device = ManagedDevice {
                            vendor_id,
                            product_id,
                            name: device_info.product_string().unwrap_or("Unknown").to_string(),
                            state: DeviceState::Active,
                        };
                        
                        // Add to device list
                        {
                            let mut devices = self.devices.lock().unwrap();
                            devices.push(managed_device);
                        }
                        
                        // Send device connected event
                        let _ = self.event_sender.send(HidEvent::DeviceConnected {
                            vendor_id,
                            product_id,
                        });
                        self.start_reader_thread(vendor_id, product_id, device);
                    }
                    Err(e) => {
                        warn!("Failed to open HID device {:04x}:{:04x}: {}", vendor_id, product_id, e);
                    }
                }
            }
        }
        
        info!("HID device scan complete");
        Ok(())
    }

    fn start_reader_thread(&self, vendor_id: u16, product_id: u16, device: HidDevice) {
        let event_sender = self.event_sender.clone();
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let key = (vendor_id, product_id);
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 64];
            loop {
                if *stop_flag_clone.lock().unwrap() {
                    break;
                }
                match device.read_timeout(&mut buf, 100) {
                    Ok(len) if len > 0 => {
                        let data = buf[..len].to_vec();
                        let _ = event_sender.send(HidEvent::InputReport {
                            vendor_id,
                            product_id,
                            data,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("HID read error for {:04x}:{:04x}: {}", vendor_id, product_id, e);
                        let _ = event_sender.send(HidEvent::DeviceDisconnected { vendor_id, product_id });
                        break;
                    }
                }
            }
        });
        let mut readers = self.readers.lock().unwrap();
        readers.insert(key, DeviceReaderHandle { stop_flag, thread_handle: Some(handle) });
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

    // Start monitoring HID devices (event-driven)
    pub fn start_monitoring(&self) {
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();
        let devices = self.devices.clone();
        let readers = self.readers.clone();
        let api = self.api.clone();
        thread::spawn(move || {
            info!("HID device monitoring thread started");
            let mut known_devices = HashMap::new();
            while *running.lock().unwrap() {
                thread::sleep(Duration::from_millis(1000));
                let mut current_devices = HashMap::new();
                let api = api.lock().unwrap();
                for device_info in api.device_list() {
                    let vendor_id = device_info.vendor_id();
                    let product_id = device_info.product_id();
                    if HidDeviceManager::is_interesting_device(vendor_id, product_id) {
                        current_devices.insert((vendor_id, product_id), device_info.path().to_owned());
                        if !known_devices.contains_key(&(vendor_id, product_id)) {
                            info!("Hotplug: New HID device {:04x}:{:04x}", vendor_id, product_id);
                            match device_info.open_device(&api) {
                                Ok(device) => {
                                    let _ = event_sender.send(HidEvent::DeviceConnected { vendor_id, product_id });
                                    // Start reader thread for new device
                                    let mut readers = readers.lock().unwrap();
                                    let stop_flag = Arc::new(Mutex::new(false));
                                    let stop_flag_clone = stop_flag.clone();
                                    let event_sender2 = event_sender.clone();
                                    let handle = thread::spawn(move || {
                                        let mut buf = [0u8; 64];
                                        loop {
                                            if *stop_flag_clone.lock().unwrap() { break; }
                                            match device.read_timeout(&mut buf, 100) {
                                                Ok(len) if len > 0 => {
                                                    let data = buf[..len].to_vec();
                                                    let _ = event_sender2.send(HidEvent::InputReport {
                                                        vendor_id,
                                                        product_id,
                                                        data,
                                                    });
                                                }
                                                Ok(_) => {}
                                                Err(e) => {
                                                    error!("HID read error for {:04x}:{:04x}: {}", vendor_id, product_id, e);
                                                    let _ = event_sender2.send(HidEvent::DeviceDisconnected { vendor_id, product_id });
                                                    break;
                                                }
                                            }
                                        }
                                    });
                                    readers.insert((vendor_id, product_id), DeviceReaderHandle { stop_flag, thread_handle: Some(handle) });
                                }
                                Err(e) => {
                                    warn!("Failed to open HID device {:04x}:{:04x}: {}", vendor_id, product_id, e);
                                }
                            }
                        }
                    }
                }
                // Remove devices that are no longer present
                for (key, _) in known_devices.iter() {
                    if !current_devices.contains_key(key) {
                        info!("Hotplug: HID device removed {:04x}:{:04x}", key.0, key.1);
                        let _ = event_sender.send(HidEvent::DeviceDisconnected { vendor_id: key.0, product_id: key.1 });
                        let mut readers = readers.lock().unwrap();
                        if let Some(reader) = readers.remove(key) {
                            *reader.stop_flag.lock().unwrap() = true;
                            if let Some(handle) = reader.thread_handle {
                                let _ = handle.join();
                            }
                        }
                    }
                }
                known_devices = current_devices;
            }
            info!("HID device monitoring thread stopped");
        });
    }

    // Stop monitoring
    pub fn stop_monitoring(&self) {
        *self.running.lock().unwrap() = false;
        // Stop all reader threads
        let mut readers = self.readers.lock().unwrap();
        for (_, reader) in readers.drain() {
            *reader.stop_flag.lock().unwrap() = true;
            if let Some(handle) = reader.thread_handle {
                let _ = handle.join();
            }
        }
    }

    // Get all managed devices
    pub fn get_devices(&self) -> Vec<ManagedDevice> {
        self.devices.lock().unwrap().clone()
    }

    // Update device state
    pub fn update_device_state(&self, vendor_id: u16, product_id: u16, state: DeviceState) {
        let mut devices = self.devices.lock().unwrap();
        for device in devices.iter_mut() {
            if device.vendor_id == vendor_id && device.product_id == product_id {
                device.state = state;
                break;
            }
        }
    }
}

// Trait for handling HID events
#[async_trait::async_trait]
pub trait HidEventHandler: Send {
    async fn handle_event(&mut self, event: HidEvent);
}

// A basic implementation that just logs events
pub struct BasicHidEventHandler {}

#[async_trait::async_trait]
impl HidEventHandler for BasicHidEventHandler {
    async fn handle_event(&mut self, event: HidEvent) {
        match event {
            HidEvent::DeviceConnected { vendor_id, product_id } => {
                info!("[HID] Device connected: {:04x}:{:04x}", vendor_id, product_id);
            }
            HidEvent::DeviceDisconnected { vendor_id, product_id } => {
                info!("[HID] Device disconnected: {:04x}:{:04x}", vendor_id, product_id);
            }
            HidEvent::InputReport { vendor_id, product_id, data } => {
                debug!("[HID] Input report from {:04x}:{:04x}: {:?}", vendor_id, product_id, data);
            }
            HidEvent::Error { vendor_id, product_id, error } => {
                error!("[HID] Error for {:04x}:{:04x}: {}", vendor_id, product_id, error);
            }
        }
    }
}

// Event loop for processing HID events using a trait object
pub async fn process_hid_events(mut handler: Box<dyn HidEventHandler>, mut event_receiver: mpsc::UnboundedReceiver<HidEvent>) {
    while let Some(event) = event_receiver.recv().await {
        handler.handle_event(event).await;
    }
} 