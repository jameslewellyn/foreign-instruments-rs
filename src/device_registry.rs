use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::{info, warn, error};
use rusb::{Device, DeviceDescriptor};
use crate::types::foreign_instruments_types::{DeviceState, ManagedDevice};

#[derive(Debug)]
pub struct DeviceRegistry {
    devices: Arc<Mutex<HashMap<String, ManagedDevice>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_device(&self, device: Device<rusb::Context>, desc: DeviceDescriptor) {
        let device_key = format!("{:4x}:{:4}", desc.vendor_id(), desc.product_id());
        let device_name = self.get_device_name(&desc);
        
        let managed_device = ManagedDevice {
            name: device_name.clone(),
            vendor_id: desc.vendor_id(),
            product_id: desc.product_id(),
            state: DeviceState::Active,
        };

        let mut devices = self.devices.lock().unwrap();
        if devices.contains_key(&device_key) {
            warn!("Device {} already exists in registry", device_key);
            return;
        }
        devices.insert(device_key.clone(), managed_device);
        info!("Added device to registry: {} (name: {}, key: {})", device_key, device_name, device_key);
    }

    pub fn remove_device(&self, vendor_id: u16, product_id: u16) {
        let device_key = format!("{:04x}:{:04x}", vendor_id, product_id);
        
        let mut devices = self.devices.lock().unwrap();
        if let Some(device) = devices.remove(&device_key) {
            info!("Removed device from registry: {} (key: {})", device.name, device_key);
        } else {
            warn!("Attempted to remove non-existent device: {}", device_key);
        }
    }

    pub fn update_device_state(&self, vendor_id: u16, product_id: u16, state: DeviceState) {
        let device_key = format!("{:04x}:{:04x}", vendor_id, product_id);
        
        let mut devices = self.devices.lock().unwrap();
        if let Some(device) = devices.get_mut(&device_key) {
            let old_state = std::mem::replace(&mut device.state, state);
            info!("Updated device state: {} (key: {}) {:?} -> {:?}", device.name, device_key, old_state, device.state);
            if let DeviceState::Error(ref error_msg) = device.state {
                error!("Device error: {} - device: {}, error: {}", error_msg, device.name, device_key);
            }
        } else {
            warn!("Attempted to update state for non-existent device: {}", device_key);
        }
    }

    pub fn get_active_devices(&self) -> Vec<ManagedDevice> {
        let devices = self.devices.lock().unwrap();
        devices
            .values()
            .filter(|device| matches!(device.state, DeviceState::Active))
            .cloned()
            .collect()
    }

    pub fn get_all_devices(&self) -> Vec<ManagedDevice> {
        let devices = self.devices.lock().unwrap();
        devices.values().cloned().collect()
    }

    pub fn device_exists(&self, vendor_id: u16, product_id: u16) -> bool {
        let device_key = format!("{:04x}:{:04x}", vendor_id, product_id);
        let devices = self.devices.lock().unwrap();
        devices.contains_key(&device_key)
    }

    pub fn get_device(&self, vendor_id: u16, product_id: u16) -> Option<ManagedDevice> {
        let device_key = format!("{:04x}:{:04x}", vendor_id, product_id);
        let devices = self.devices.lock().unwrap();
        devices.get(&device_key).cloned()
    }

    fn get_device_name(&self, desc: &DeviceDescriptor) -> String {
        // Try to get the actual device name, fallback to vendor:product format
        format!("Device {:4x}:{:4}", desc.vendor_id(), desc.product_id())
    }

    pub fn print_status(&self) {
        let devices = self.get_all_devices();
        info!("Device Registry Status:");
        info!("  Total devices: {}", devices.len());
        
        let active_count = devices.iter().filter(|d| matches!(d.state, DeviceState::Active)).count();
        let error_count = devices.iter().filter(|d| matches!(d.state, DeviceState::Error(_))).count();
        let disconnected_count = devices.iter().filter(|d| matches!(d.state, DeviceState::Disconnected)).count();
        
        info!("  Active: {}, Disconnected: {}, Errors: {}", active_count, disconnected_count, error_count);
        
        for device in devices {
            match &device.state {
                DeviceState::Active => info!("  ✓ {} ({:04x}:{:04x})", device.name, device.vendor_id, device.product_id),
                DeviceState::Disconnected => warn!("  ✗ {} ({:04x}:{:04x}) - Disconnected", device.name, device.vendor_id, device.product_id),
                DeviceState::Error(msg) => error!("  ⚠ {} ({:04x}:{:04x}) - Error: {}", device.name, device.vendor_id, device.product_id, msg),
            }
        }
    }
} 