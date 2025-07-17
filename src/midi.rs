use midir::{MidiOutput, MidiOutputConnection};
use midir::os::unix::VirtualOutput;
use log::{info, error, debug};
use std::collections::HashMap;

// MIDI message types
#[derive(Debug, Clone)]
pub enum MidiMessage {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    ProgramChange { channel: u8, program: u8 },
    PitchBend { channel: u8, value: u16 },
    Aftertouch { channel: u8, pressure: u8 },
}

impl MidiMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            MidiMessage::NoteOn { channel, note, velocity } => {
                vec![0x90 | (channel & 0x0F), *note, *velocity]
            }
            MidiMessage::NoteOff { channel, note, velocity } => {
                vec![0x80 | (channel & 0x0F), *note, *velocity]
            }
            MidiMessage::ControlChange { channel, controller, value } => {
                vec![0xB0 | (channel & 0x0F), *controller, *value]
            }
            MidiMessage::ProgramChange { channel, program } => {
                vec![0xC0 | (channel & 0x0F), *program]
            }
            MidiMessage::PitchBend { channel, value } => {
                let lsb = (value & 0x7F) as u8;
                let msb = ((value >> 7) & 0x7F) as u8;
                vec![0xE0 | (channel & 0x0F), lsb, msb]
            }
            MidiMessage::Aftertouch { channel, pressure } => {
                vec![0xD0 | (channel & 0x0F), *pressure]
            }
        }
    }
}

// USB message types that can be translated to MIDI
#[derive(Debug, Clone)]
pub enum UsbMessage {
    Button { button_id: u8, pressed: bool },
    Knob { knob_id: u8, value: u8 },
    Fader { fader_id: u8, value: u8 },
    Pad { pad_id: u8, velocity: u8, pressed: bool },
    SmartStrip { strip_id: u8, value: u8 },
    Encoder { encoder_id: u8, delta: i8 },
}

// Translation configuration for mapping USB controls to MIDI
#[derive(Debug, Clone)]
pub struct MidiMapping {
    pub channel: u8,
    pub mappings: HashMap<String, MidiControlMapping>,
}

#[derive(Debug, Clone)]
pub enum MidiControlMapping {
    Note { note: u8, velocity: u8 },
    ControlChange { controller: u8 },
    ProgramChange { program: u8 },
    PitchBend,
    Aftertouch,
}

// Trait for translating USB messages to MIDI
pub trait UsbToMidiTranslator: Send {
    fn translate(&self, usb_msg: &UsbMessage, mapping: &MidiMapping) -> Option<MidiMessage>;
    fn get_default_mapping(&self, device_vid: u16, device_pid: u16) -> MidiMapping;
}

// Implementation for Native Instruments devices
pub struct NativeInstrumentsTranslator;

impl UsbToMidiTranslator for NativeInstrumentsTranslator {
    fn translate(&self, usb_msg: &UsbMessage, mapping: &MidiMapping) -> Option<MidiMessage> {
        match usb_msg {
            UsbMessage::Button { button_id, pressed } => {
                let key = format!("button_{}", button_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::Note { note, velocity } => {
                            let midi_velocity = if *pressed { *velocity } else { 0 };
                            Some(MidiMessage::NoteOn {
                                channel: mapping.channel,
                                note: *note,
                                velocity: midi_velocity,
                            })
                        }
                        MidiControlMapping::ControlChange { controller } => {
                            Some(MidiMessage::ControlChange {
                                channel: mapping.channel,
                                controller: *controller,
                                value: if *pressed { 127 } else { 0 },
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            UsbMessage::Knob { knob_id, value } => {
                let key = format!("knob_{}", knob_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::ControlChange { controller } => {
                            Some(MidiMessage::ControlChange {
                                channel: mapping.channel,
                                controller: *controller,
                                value: *value,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            UsbMessage::Pad { pad_id, velocity, pressed } => {
                let key = format!("pad_{}", pad_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::Note { note, velocity: _default_vel } => {
                            let midi_velocity = if *pressed { *velocity } else { 0 };
                            Some(MidiMessage::NoteOn {
                                channel: mapping.channel,
                                note: *note,
                                velocity: midi_velocity,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            UsbMessage::Fader { fader_id, value } => {
                let key = format!("fader_{}", fader_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::ControlChange { controller } => {
                            Some(MidiMessage::ControlChange {
                                channel: mapping.channel,
                                controller: *controller,
                                value: *value,
                            })
                        }
                        MidiControlMapping::PitchBend => {
                            let pitch_value = (*value as u16) << 7; // Convert 7-bit to 14-bit
                            Some(MidiMessage::PitchBend {
                                channel: mapping.channel,
                                value: pitch_value,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            UsbMessage::SmartStrip { strip_id, value } => {
                let key = format!("strip_{}", strip_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::ControlChange { controller } => {
                            Some(MidiMessage::ControlChange {
                                channel: mapping.channel,
                                controller: *controller,
                                value: *value,
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            UsbMessage::Encoder { encoder_id, delta } => {
                let key = format!("encoder_{}", encoder_id);
                if let Some(control_mapping) = mapping.mappings.get(&key) {
                    match control_mapping {
                        MidiControlMapping::ControlChange { controller } => {
                            // For encoders, we'd need to track the current value
                            // For now, just send the delta as a control change
                            Some(MidiMessage::ControlChange {
                                channel: mapping.channel,
                                controller: *controller,
                                value: (*delta as u8).saturating_add(64), // Center at 64
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }

    fn get_default_mapping(&self, device_vid: u16, device_pid: u16) -> MidiMapping {
        let mut mappings = HashMap::new();
        
        // Maschine Jam default mapping
        if device_vid == 0x17CC && device_pid == 0x1500 {
            // Buttons for transport controls
            mappings.insert("button_0".to_string(), MidiControlMapping::ControlChange { controller: 0x7B }); // Stop
            mappings.insert("button_1".to_string(), MidiControlMapping::ControlChange { controller: 0x7C }); // Play
            mappings.insert("button_2".to_string(), MidiControlMapping::ControlChange { controller: 0x7D }); // Record
            
            // Pads for drum hits
            for i in 0..16 {
                mappings.insert(format!("pad_{}", i), MidiControlMapping::Note { 
                    note: 36 + i, // C2 to D#3
                    velocity: 100 
                });
            }
            
            // Knobs for CC controls
            for i in 0..8 {
                mappings.insert(format!("knob_{}", i), MidiControlMapping::ControlChange { 
                    controller: 0x10 + i // CC 16-23
                });
            }
            
            // Smart strip for pitch bend
            mappings.insert("strip_0".to_string(), MidiControlMapping::PitchBend);
        }
        
        MidiMapping {
            channel: 0,
            mappings,
        }
    }
}

pub struct MidiTranslator {
    conn_out: MidiOutputConnection,
    pub translator: Box<dyn UsbToMidiTranslator>,
}

impl MidiTranslator {
    pub fn new(port_name: &str) -> Result<Self, String> {
        let midi_out = MidiOutput::new("foreign-instruments-midi").map_err(|e| e.to_string())?;
        let conn_out = midi_out.create_virtual(port_name).map_err(|e| e.to_string())?;
        info!("Virtual MIDI port '{}' created", port_name);
        
        let translator = Box::new(NativeInstrumentsTranslator);
        
        Ok(MidiTranslator { conn_out, translator })
    }

    pub fn send_midi_message(&mut self, message: &MidiMessage) -> Result<(), String> {
        let bytes = message.to_bytes();
        debug!("Sending MIDI message: {:?} -> {:02X?}", message, bytes);
        
        self.conn_out.send(&bytes).map_err(|e| e.to_string())?;
        info!("Sent MIDI message: {:?}", message);
        Ok(())
    }

    pub fn translate_and_send(&mut self, usb_msg: &UsbMessage, mapping: &MidiMapping) -> Result<(), String> {
        if let Some(midi_msg) = self.translator.translate(usb_msg, mapping) {
            self.send_midi_message(&midi_msg)
        } else {
            debug!("No translation found for USB message: {:?}", usb_msg);
            Ok(())
        }
    }

    pub fn send_test_message(&mut self) {
        let message = MidiMessage::NoteOn { channel: 0, note: 60, velocity: 64 };
        if let Err(e) = self.send_midi_message(&message) {
            error!("Failed to send test MIDI message: {}", e);
        }
    }
} 