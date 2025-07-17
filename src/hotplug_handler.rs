use rusb::{Context, HotplugBuilder, UsbContext, Hotplug};
use std::sync::Arc;
use log::{info, warn, error};
use crate::device_registry::DeviceRegistry;
use crate::types::foreign_instruments_types::DeviceState;

pub struct HotplugHandler {
    context: Context,
    registry: Arc<DeviceRegistry>,
    _registrations: Vec<rusb::Registration<Context>>, // Keep registrations alive
}

impl HotplugHandler {
    pub fn new(registry: Arc<DeviceRegistry>) -> Result<Self, rusb::Error> {
        let context = Context::new()?;
        Ok(Self {
            context,
            registry,
            _registrations: Vec::new(),
        })
    }

    pub fn register_hotplug_callbacks(&mut self) -> Result<(), rusb::Error> {
        if !rusb::has_hotplug() {
            warn!("Hotplug support is not available on this platform");
            return Ok(());
        }

        let registry = Arc::clone(&self.registry);
        
        // Create arrival handler
        let arrival_handler = Box::new(DeviceArrivalHandler {
            registry: registry.clone(),
        });
        
        // Create departure handler  
        let departure_handler = Box::new(DeviceDepartureHandler {
            registry: registry.clone(),
        });

        // Register arrival callback
        let arrival_registration = HotplugBuilder::new()
            .vendor_id(0x17cc)
            .register(&self.context, arrival_handler)?;
            
        // Register departure callback
        let departure_registration = HotplugBuilder::new()
            .vendor_id(0x17cc)
            .register(&self.context, departure_handler)?;

        // Store registrations to keep them alive
        self._registrations.push(arrival_registration);
        self._registrations.push(departure_registration);
        
        info!("Hotplug callbacks registered successfully");
        Ok(())
    }

    pub fn scan_initial_devices(&self) -> Result<(), rusb::Error> {
        info!("Scanning for initial devices...");
        
        for device in self.context.devices()?.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to get device descriptor: {}", e);
                    continue;
                }
            };

            // Check if this is a device we're interested in
            if self.is_interesting_device(desc.vendor_id()) {
                info!("Found interesting device: {:04x}:{:04x}", 
                      desc.vendor_id(), desc.product_id());
                
                // Only add if not already in registry
                if !self.registry.device_exists(desc.vendor_id(), desc.product_id()) {
                    self.registry.add_device(device, desc);
                }
            }
        }

        self.registry.print_status();
        Ok(())
    }

    pub fn poll_devices(&self) -> Result<(), rusb::Error> {
        // Poll all devices in registry to check their status
        let devices = self.registry.get_all_devices();
        
        for device in devices {
            // Try to open the device to check if it's still accessible
            let device_key = format!("{:04x}:{:04x}", device.vendor_id, device.product_id);
            
            // This is a simplified check - in a real implementation you'd want to
            // actually try to communicate with the device
            if let Some(usb_device) = self.find_device_by_vid_pid(device.vendor_id, device.product_id) {
                // Device still exists, ensure it's marked as active
                if !matches!(device.state, DeviceState::Active) {
                    self.registry.update_device_state(
                        device.vendor_id, 
                        device.product_id, 
                        DeviceState::Active
                    );
                }
            } else {
                // Device not found, mark as disconnected
                if matches!(device.state, DeviceState::Active) {
                    self.registry.update_device_state(
                        device.vendor_id, 
                        device.product_id, 
                        DeviceState::Disconnected
                    );
                }
            }
        }
        
        Ok(())
    }

    fn is_interesting_device(&self, vid: u16) -> bool {
        vid == 0x17cc
            || vid == 0x0763 // M-Audio
            || vid == 0x1235 // Focusrite
            || vid == 0x1bcf // Arturia
    }

    fn find_device_by_vid_pid(&self, vid: u16, pid: u16) -> Option<rusb::Device<Context>> {
        for device in self.context.devices().ok()?.iter() {
            if let Ok(desc) = device.device_descriptor() {
                if desc.vendor_id() == vid && desc.product_id() == pid {
                    return Some(device);
                }
            }
        }
        None
    }

    pub fn get_context(&self) -> &Context {
        &self.context
    }
}

// Handler for device arrival events
struct DeviceArrivalHandler {
    registry: Arc<DeviceRegistry>,
}

impl Hotplug<Context> for DeviceArrivalHandler {
    fn device_arrived(&mut self, device: rusb::Device<Context>) {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to get device descriptor on arrival: {}", e);
                return;
            }
        };
        info!("Hotplug: Device connected - {:04x}:{:04x}", desc.vendor_id(), desc.product_id());
        self.registry.add_device(device, desc);
        self.registry.print_status();
    }

    fn device_left(&mut self, _device: rusb::Device<Context>) {
        // This handler is only for arrivals, departures are handled separately
    }
}

// Handler for device departure events
struct DeviceDepartureHandler {
    registry: Arc<DeviceRegistry>,
}

impl Hotplug<Context> for DeviceDepartureHandler {
    fn device_arrived(&mut self, _device: rusb::Device<Context>) {
        // This handler is only for departures, arrivals are handled separately
    }

    fn device_left(&mut self, device: rusb::Device<Context>) {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to get device descriptor on departure: {}", e);
                return;
            }
        };
        info!("Hotplug: Device disconnected - {:04x}:{:04x}", desc.vendor_id(), desc.product_id());
        self.registry.remove_device(desc.vendor_id(), desc.product_id());
        self.registry.print_status();
    }
} 