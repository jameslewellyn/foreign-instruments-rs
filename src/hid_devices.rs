use hidapi::{HidApi, HidDevice, HidError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use log::{info, warn, error, debug};
use tokio::sync::mpsc;
use crate::types::foreign_instruments_types::{DeviceState, ManagedDevice};
use crate::midi_output::MidiOutputManager;

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
    api: HidApi,
    devices: Arc<Mutex<Vec<ManagedDevice>>>,
    event_sender: mpsc::UnboundedSender<HidEvent>,
    running: Arc<Mutex<bool>>,
    readers: Arc<Mutex<HashMap<(u16, u16), DeviceReaderHandle>>>,
}

impl HidDeviceManager {
    pub fn new(event_sender: mpsc::UnboundedSender<HidEvent>) -> Result<Self, HidError> {
        let api = HidApi::new()?;
        Ok(Self {
            api,
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

    // Scan for initial HID devices
    pub fn scan_initial_devices(&self) -> Result<(), HidError> {
        info!("Scanning for initial HID devices...");
        
        for device_info in self.api.device_list() {
            let vendor_id = device_info.vendor_id();
            let product_id = device_info.product_id();
            
            if Self::is_interesting_device(vendor_id, product_id) {
                info!("Found interesting HID device: {:04x}:{:04x} - {}", 
                      vendor_id, product_id, device_info.product_string().unwrap_or("Unknown"));
                
                // Try to open the device
                match device_info.open_device(&self.api) {
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

// HID event handler for specific devices
pub struct HidEventHandler {
    event_receiver: mpsc::UnboundedReceiver<HidEvent>,
    midi_output: Option<Arc<MidiOutputManager>>,
}

impl HidEventHandler {
    pub fn new(event_receiver: mpsc::UnboundedReceiver<HidEvent>) -> Self {
        Self {
            event_receiver,
            midi_output: None,
        }
    }
    pub fn new_with_midi(event_receiver: mpsc::UnboundedReceiver<HidEvent>, midi_output: Arc<MidiOutputManager>) -> Self {
        Self {
            event_receiver,
            midi_output: Some(midi_output),
        }
    }

    pub fn set_midi_sender(&mut self, sender: mpsc::UnboundedSender<Vec<u8>>) {
        self.midi_output = Some(Arc::new(MidiOutputManager::new(sender)));
    }

    // Start processing HID events
    pub async fn start_processing(&mut self) {
        info!("HID event processor started");
        
        while let Some(event) = self.event_receiver.recv().await {
            match event {
                HidEvent::DeviceConnected { vendor_id, product_id } => {
                    info!("HID device connected: {:04x}:{:04x}", vendor_id, product_id);
                    self.handle_device_connected(vendor_id, product_id).await;
                }
                HidEvent::DeviceDisconnected { vendor_id, product_id } => {
                    info!("HID device disconnected: {:04x}:{:04x}", vendor_id, product_id);
                    self.handle_device_disconnected(vendor_id, product_id).await;
                }
                HidEvent::InputReport { vendor_id, product_id, data } => {
                    debug!("HID input report from {:04x}:{:04x}: {:?}", vendor_id, product_id, data);
                    self.handle_input_report(vendor_id, product_id, data).await;
                }
                HidEvent::Error { vendor_id, product_id, error } => {
                    error!("HID error for {:04x}:{:04x}: {}", vendor_id, product_id, error);
                    self.handle_error(vendor_id, product_id, error).await;
                }
            }
        }
        
        info!("HID event processor stopped");
    }

    async fn handle_device_connected(&self, vendor_id: u16, product_id: u16) {
        // Handle device-specific initialization
        if HidDeviceManager::is_native_instruments_device(vendor_id, product_id) {
            info!("Native Instruments device connected: {:04x}:{:04x}", vendor_id, product_id);
            
            // Start device-specific monitoring
            match product_id {
                0x1500 => {
                    info!("Maschine Jam detected - starting HID monitoring");
                    // TODO: Start Maschine Jam specific HID monitoring
                }
                _ => {
                    info!("Unknown Native Instruments device: {:04x}:{:04x}", vendor_id, product_id);
                }
            }
        }
    }

    async fn handle_device_disconnected(&self, vendor_id: u16, product_id: u16) {
        info!("Device disconnected: {:04x}:{:04x}", vendor_id, product_id);
        // Handle device cleanup
    }

    async fn handle_input_report(&self, vendor_id: u16, product_id: u16, data: Vec<u8>) {
        // Convert HID input report to MIDI
        if let Some(midi_data) = self.translate_hid_to_midi(vendor_id, product_id, &data) {
            if let Some(ref midi_output) = self.midi_output {
                midi_output.send(&midi_data);
            }
        }
    }

    async fn handle_error(&self, vendor_id: u16, product_id: u16, error: String) {
        error!("HID error for device {:04x}:{:04x}: {}", vendor_id, product_id, error);
        // Handle device errors
    }

    // Translate HID input report to MIDI
    fn translate_hid_to_midi(&self, vendor_id: u16, product_id: u16, data: &[u8]) -> Option<Vec<u8>> {
        if HidDeviceManager::is_native_instruments_device(vendor_id, product_id) {
            match product_id {
                0x1500 => self.translate_maschine_jam_to_midi(data),
                _ => None,
            }
        } else {
            None
        }
    }

    // Maschine Jam specific HID to MIDI translation
    fn translate_maschine_jam_to_midi(&self, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 2 {
            return None;
        }

        // Basic Maschine Jam HID to MIDI translation
        // This is a simplified implementation - real implementation would be more complex
        let report_id = data[0];
        
        match report_id {
            0x01 => {
                // Button press/release
                if data.len() >= 3 {
                    let button = data[1];
                    let pressed = data[2] > 0;
                    
                    if pressed {
                        // Note On
                        Some(vec![0x90, button, 0x7F])
                    } else {
                        // Note Off
                        Some(vec![0x80, button, 0x00])
                    }
                } else {
                    None
                }
            }
            0x02 => {
                // Encoder/touch strip
                if data.len() >= 3 {
                    let controller = data[1];
                    let value = data[2];
                    
                    // Control Change
                    Some(vec![0xB0, controller, value])
                } else {
                    None
                }
            }
            _ => {
                debug!("Unknown Maschine Jam report ID: 0x{:02X}", report_id);
                None
            }
        }
    }
}

// TODO: Implement proper HID device reading with proper device management
// For now, we'll use a simplified approach without continuous reading 