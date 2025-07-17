use serde::Deserialize;
use toml::Value;

#[derive(Debug, Clone)]
pub enum PatternByte {
    Exact(u8),
    Wildcard, // matches any value
    Range { min: u8, max: u8 },
}

impl<'de> Deserialize<'de> for PatternByte {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) if s == "any" => Ok(PatternByte::Wildcard),
            Value::Integer(n) => {
                if n < 0 || n > 255 {
                    return Err(Error::custom("Byte value out of range"));
                }
                Ok(PatternByte::Exact(n as u8))
            }
            Value::Table(mut obj) => {
                let min = obj.remove("min")
                    .and_then(|v| v.as_integer())
                    .ok_or_else(|| Error::custom("Missing or invalid min in range"))?;
                let max = obj.remove("max")
                    .and_then(|v| v.as_integer())
                    .ok_or_else(|| Error::custom("Missing or invalid max in range"))?;
                if min < 0 || min > 255 || max < 0 || max > 255 {
                    return Err(Error::custom("Invalid range values"));
                }
                Ok(PatternByte::Range { min: min as u8, max: max as u8 })
            }
            _ => Err(Error::custom("Invalid pattern byte format")),
        }
    }
}

impl PatternByte {
    pub fn matches(&self, byte: u8) -> bool {
        match self {
            PatternByte::Exact(expected) => *expected == byte,
            PatternByte::Wildcard => true,
            PatternByte::Range { min, max } => byte >= *min && byte <= *max,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MidiMapping {
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub usb_pattern: Vec<PatternByte>,
    pub midi_message: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MidiMappingConfig {
    pub mapping: Vec<MidiMapping>,
} 