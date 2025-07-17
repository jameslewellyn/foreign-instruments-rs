use midir::{MidiOutput, MidiOutputConnection};
use log::{info, error};
use crate::midi_mapping::{MidiMapping, MidiMappingConfig, PatternByte};
use midir::os::unix::VirtualOutput;

pub struct MidiTranslator {
    conn_out: MidiOutputConnection,
}

impl MidiTranslator {
    pub fn new(port_name: &str) -> Result<Self, String> {
        let midi_out = MidiOutput::new("foreign-instruments-midi").map_err(|e| e.to_string())?;
        let conn_out = midi_out.create_virtual(port_name).map_err(|e| e.to_string())?;
        info!("Virtual MIDI port '{}' created", port_name);
        Ok(MidiTranslator { conn_out })
    }

    pub fn send_test_message(&mut self) {
        // Send a simple Note On message (Middle C, velocity 64)
        let message = [0x90, 60, 64];
        if let Err(e) = self.conn_out.send(&message) {
            error!("Failed to send MIDI message: {}", e);
        } else {
            info!("Sent test MIDI Note On message");
        }
    }

    pub fn send_midi_message(&mut self, message: &[u8]) {
        if let Err(e) = self.conn_out.send(message) {
            error!("Failed to send MIDI message: {}", e);
        }
    }
}

/// Trait for translating USB messages to MIDI messages
pub trait UsbToMidiTranslator {
    /// Translate a raw USB message (as bytes) to zero or more MIDI messages
    fn translate_usb_to_midi(&self, usb_data: &[u8], vendor_id: Option<u16>, product_id: Option<u16>) -> Vec<Vec<u8>>;
}

/// A basic passthrough translator (for testing)
pub struct PassthroughUsbToMidi;

impl UsbToMidiTranslator for PassthroughUsbToMidi {
    fn translate_usb_to_midi(&self, usb_data: &[u8], _vendor_id: Option<u16>, _product_id: Option<u16>) -> Vec<Vec<u8>> {
        // For now, just wrap the USB data as a single MIDI message (not valid MIDI, just for testing)
        vec![usb_data.to_vec()]
    }
}

/// Mapping-based translator
pub struct MappingUsbToMidi {
    pub mappings: Vec<MidiMapping>,
}

impl UsbToMidiTranslator for MappingUsbToMidi {
    fn translate_usb_to_midi(&self, usb_data: &[u8], vendor_id: Option<u16>, product_id: Option<u16>) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        for mapping in &self.mappings {
            let matches_vendor = mapping.vendor_id.map_or(true, |vid| Some(vid) == vendor_id);
            let matches_product = mapping.product_id.map_or(true, |pid| Some(pid) == product_id);
            let matches_pattern = self.matches_pattern(usb_data, &mapping.usb_pattern);
            if matches_vendor && matches_product && matches_pattern {
                result.push(mapping.midi_message.clone());
            }
        }
        result
    }
}

impl MappingUsbToMidi {
    pub fn from_config_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: MidiMappingConfig = toml::from_str(&content).map_err(|e| e.to_string())?;
        Ok(MappingUsbToMidi { mappings: config.mapping })
    }

    fn matches_pattern(&self, usb_data: &[u8], pattern: &[PatternByte]) -> bool {
        if usb_data.len() < pattern.len() {
            return false;
        }
        
        for (i, pattern_byte) in pattern.iter().enumerate() {
            if !pattern_byte.matches(usb_data[i]) {
                return false;
            }
        }
        true
    }
} 