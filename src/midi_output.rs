use midir::{MidiOutput, MidiOutputConnection, InitError, ConnectError, PortInfoError};
use std::sync::{Arc, Mutex};
use log::{info, error};

pub struct MidiOutputManager {
    conn: Arc<Mutex<MidiOutputConnection>>,
}

impl MidiOutputManager {
    pub fn new(port_name: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let midi_out = MidiOutput::new("Foreign Instruments MIDI Output")?;
        let conn = midi_out.create_virtual(port_name)?;
        info!("Created virtual MIDI output port: {}", port_name);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn send(&self, data: &[u8]) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.send(data) {
            error!("Failed to send MIDI message: {}", e);
        }
    }
} 