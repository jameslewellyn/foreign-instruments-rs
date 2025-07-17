use rusb::{Context, UsbContext};

fn main() {
    println!("🔍 Testing USB Device Detection");
    println!("================================");
    
    let context = match Context::new() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Failed to create USB context: {}", e);
            return;
        }
    };
    
    let devices = match context.devices() {
        Ok(devs) => devs,
        Err(e) => {
            eprintln!("Failed to get devices: {}", e);
            return;
        }
    };
    
    println!("Found {} USB devices", devices.len());
    println!();
    
    let mut native_instruments_found = false;
    
    for device in devices.iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(e) => {
                println!("Failed to get device descriptor: {}", e);
                continue;
            }
        };
        
        let vid = desc.vendor_id();
        let pid = desc.product_id();
        
        // Check if this is Native Instruments (17cc)
        if vid == 0x17cc {
            native_instruments_found = true;
            println!("🎵 NATIVE INSTRUMENTS DEVICE FOUND:");
            println!("   Vendor ID: 0x{:04x}", vid);
            println!("   Product ID: 0x{:04x}", pid);
            println!("   Device: {:?}", device);
            
            // Try to open the device
            match device.open() {
                Ok(handle) => {
                    println!("   ✅ Successfully opened device");
                    
                    // Try to claim interface
                    match handle.claim_interface(0) {
                        Ok(_) => println!("   ✅ Successfully claimed interface 0"),
                        Err(e) => println!("   ❌ Failed to claim interface 0: {}", e),
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to open device: {}", e);
                }
            }
            println!();
        }
        
        // Also show other interesting devices
        if vid == 0x0763 || vid == 0x1235 || vid == 0x1bcf {
            println!("🎹 Other MIDI device: 0x{:04x}:0x{:04x}", vid, pid);
        }
    }
    
    if !native_instruments_found {
        println!("❌ No Native Instruments devices found!");
    } else {
        println!("✅ Native Instruments device(s) detected successfully!");
    }
    
    println!();
    println!("Now you can test hotplug events:");
    println!("1. Disconnect your Maschine Jam");
    println!("2. Wait 2-3 seconds");
    println!("3. Reconnect your Maschine Jam");
    println!("4. Watch for hotplug events in the main application");
} 