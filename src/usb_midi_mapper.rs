use crate::midi::{MidiMessage, UsbMessage, MidiMapping, MidiTranslator};
use log::{info, debug, warn};
use std::collections::HashMap;

/// Parser for Maschine Jam USB data packets
pub struct MaschineJamParser {
    /// Track button states to detect press/release
    button_states: HashMap<u8, bool>,
    /// Track last values to detect changes
    last_values: HashMap<u8, u8>,
}

impl MaschineJamParser {
    pub fn new() -> Self {
        Self {
            button_states: HashMap::new(),
            last_values: HashMap::new(),
        }
    }

    /// Parse raw USB data from Maschine Jam and convert to USB messages
    pub fn parse_usb_data(&mut self, data: &[u8]) -> Vec<UsbMessage> {
        let mut messages = Vec::new();
        
        if data.len() < 17 || data[0] != 0x01 {
            debug!("Invalid USB data format: {:02x?}", data);
            return messages;
        }

        info!("🔍 Parsing USB data: {:02x?}", data);

        // Parse the data bytes (skip header byte 0x01)
        let data_bytes = &data[1..17];
        
        // Analyze each byte position for button activity
        for (position, &value) in data_bytes.iter().enumerate() {
            let button_id = position as u8;
            
            // Check if this is a button press/release
            if self.is_button_byte(position, value) {
                let was_pressed = self.button_states.get(&button_id).unwrap_or(&false);
                let is_pressed = value != 0;
                
                if is_pressed != *was_pressed {
                    self.button_states.insert(button_id, is_pressed);
                    
                    if is_pressed {
                        messages.push(UsbMessage::Button {
                            button_id,
                            pressed: true,
                        });
                        info!("🔘 Button {} PRESSED (value: 0x{:02x})", button_id, value);
                    } else {
                        messages.push(UsbMessage::Button {
                            button_id,
                            pressed: false,
                        });
                        info!("🔘 Button {} RELEASED", button_id);
                    }
                } else if is_pressed {
                    // Log when button is held down (for debugging)
                    debug!("🔘 Button {} HELD (value: 0x{:02x})", button_id, value);
                }
            }
            
            // Check for value changes (for knobs, faders, etc.)
            if let Some(&last_value) = self.last_values.get(&button_id) {
                if value != last_value && value != 0 {
                    // This might be a knob or fader
                    if self.is_knob_byte(position, value) {
                        messages.push(UsbMessage::Knob {
                            knob_id: button_id,
                            value,
                        });
                        info!("🎛️ Knob {} value: 0x{:02x}", button_id, value);
                    } else if self.is_fader_byte(position, value) {
                        messages.push(UsbMessage::Fader {
                            fader_id: button_id,
                            value,
                        });
                        info!("🎚️ Fader {} value: 0x{:02x}", button_id, value);
                    }
                }
            }
            
            self.last_values.insert(button_id, value);
        }
        
        messages
    }

    /// Determine if a byte position represents a button
    fn is_button_byte(&self, position: usize, value: u8) -> bool {
        // Based on the data we've seen, buttons are in position 5 (6th byte, 0-indexed)
        // and have specific values like 0x10, 0x20, 0x40, etc.
        match position {
            5 => {
                // Common button values we've observed in the USB data
                matches!(value, 0x00 | 0x10 | 0x20 | 0x30 | 0x40 | 0x50 | 0x60)
            }
            _ => false,
        }
    }

    /// Determine if a byte position represents a knob
    fn is_knob_byte(&self, position: usize, _value: u8) -> bool {
        // Knobs might be in different positions - need to observe more data
        // For now, assume positions 7-15 might be knobs
        position >= 7 && position <= 15
    }

    /// Determine if a byte position represents a fader
    fn is_fader_byte(&self, position: usize, _value: u8) -> bool {
        // Faders might be in different positions - need to observe more data
        // For now, assume positions 7-15 might be faders
        position >= 7 && position <= 15
    }
}

/// MIDI mapping configuration for Maschine Jam
pub struct MaschineJamMidiMapping {
    pub mapping: MidiMapping,
}

impl MaschineJamMidiMapping {
    pub fn new() -> Self {
        let mut mappings = HashMap::new();
        
        // Map buttons to MIDI controls
        // Based on observed data, position 5 and 6 seem to be buttons
        
        // Transport controls (position 5)
        mappings.insert("button_5".to_string(), crate::midi::MidiControlMapping::ControlChange { controller: 0x7B }); // Stop
        mappings.insert("button_6".to_string(), crate::midi::MidiControlMapping::ControlChange { controller: 0x7C }); // Play
        
        // Additional buttons (position 6)
        mappings.insert("button_7".to_string(), crate::midi::MidiControlMapping::ControlChange { controller: 0x7D }); // Record
        mappings.insert("button_8".to_string(), crate::midi::MidiControlMapping::ControlChange { controller: 0x7E }); // Loop
        
        // Map knobs to CC controls
        for i in 0..8 {
            mappings.insert(format!("knob_{}", i), crate::midi::MidiControlMapping::ControlChange { 
                controller: 0x10 + i // CC 16-23
            });
        }
        
        // Map pads to notes (if we find pad data)
        for i in 0..16 {
            mappings.insert(format!("pad_{}", i), crate::midi::MidiControlMapping::Note { 
                note: 36 + i, // C2 to D#3
                velocity: 100 
            });
        }
        
        Self {
            mapping: MidiMapping {
                channel: 0,
                mappings,
            }
        }
    }
}

/// Main USB-to-MIDI bridge for Maschine Jam
pub struct MaschineJamMidiBridge {
    parser: MaschineJamParser,
    midi_translator: MidiTranslator,
    mapping: MaschineJamMidiMapping,
}

impl MaschineJamMidiBridge {
    pub fn new() -> Result<Self, String> {
        let midi_translator = MidiTranslator::new("Maschine Jam MIDI")?;
        let parser = MaschineJamParser::new();
        let mapping = MaschineJamMidiMapping::new();
        
        Ok(Self {
            parser,
            midi_translator,
            mapping,
        })
    }

    /// Process raw USB data and send MIDI messages
    pub fn process_usb_data(&mut self, data: &[u8]) -> Result<(), String> {
        // Parse USB data into USB messages
        let usb_messages = self.parser.parse_usb_data(data);
        
        if !usb_messages.is_empty() {
            info!("🎵 Parsed {} USB messages from data: {:02x?}", usb_messages.len(), data);
        }
        
        // Convert each USB message to MIDI and send
        for usb_msg in usb_messages {
            info!("📤 USB Message: {:?}", usb_msg);
            
            if let Err(e) = self.midi_translator.translate_and_send(&usb_msg, &self.mapping.mapping) {
                warn!("Failed to translate USB message to MIDI: {}", e);
            } else {
                info!("🎹 MIDI message sent successfully for USB message: {:?}", usb_msg);
            }
        }
        
        Ok(())
    }

    /// Send a test MIDI message
    pub fn send_test_message(&mut self) {
        self.midi_translator.send_test_message();
    }
}

/// Enhanced event handler that includes MIDI translation
pub struct MidiEnabledRusbHidEventHandler {
    midi_bridge: Option<MaschineJamMidiBridge>,
}

impl MidiEnabledRusbHidEventHandler {
    pub fn new() -> Self {
        Self {
            midi_bridge: None,
        }
    }

    pub fn with_midi_bridge(&mut self) -> Result<(), String> {
        let bridge = MaschineJamMidiBridge::new()?;
        self.midi_bridge = Some(bridge);
        Ok(())
    }

    pub async fn handle_event(&mut self, event: crate::rusb_hid_manager::RusbHidEvent) {
        info!("🎯 Event handler received event: {:?}", event);
        match event {
            crate::rusb_hid_manager::RusbHidEvent::DeviceConnected { vendor_id, product_id } => {
                info!("🎹 USB Device Connected: {:04x}:{:04x}", vendor_id, product_id);
                
                // Initialize MIDI bridge for Maschine Jam
                if vendor_id == 0x17cc && product_id == 0x1500 {
                    if let Ok(bridge) = MaschineJamMidiBridge::new() {
                        self.midi_bridge = Some(bridge);
                        info!("🎵 MIDI bridge initialized for Maschine Jam");
                    } else {
                        warn!("Failed to initialize MIDI bridge for Maschine Jam");
                    }
                }
            }
            crate::rusb_hid_manager::RusbHidEvent::DeviceDisconnected { vendor_id, product_id } => {
                info!("🔌 USB Device Disconnected: {:04x}:{:04x}", vendor_id, product_id);
                
                // Clear MIDI bridge if it was for this device
                if vendor_id == 0x17cc && product_id == 0x1500 {
                    self.midi_bridge = None;
                    info!("🎵 MIDI bridge cleared for Maschine Jam");
                }
            }
            crate::rusb_hid_manager::RusbHidEvent::InputReport { vendor_id, product_id, data } => {
                info!("📥 USB Input from {:04x}:{:04x}: {:02x?}", vendor_id, product_id, data);
                
                // Process USB data through MIDI bridge if available
                if let Some(ref mut bridge) = self.midi_bridge {
                    if let Err(e) = bridge.process_usb_data(&data) {
                        warn!("Failed to process USB data through MIDI bridge: {}", e);
                    }
                }
            }
            crate::rusb_hid_manager::RusbHidEvent::Error { vendor_id, product_id, error } => {
                log::error!("❌ USB Error for {:04x}:{:04x}: {}", vendor_id, product_id, error);
            }
        }
    }
} 