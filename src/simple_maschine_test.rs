use rusb::{Context, Direction, TransferType};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vid = 0x17cc;
    let pid = 0x1500;
    let iface = 0;

    println!("🎵 Testing Maschine Jam USB Reading");
    println!("===================================");

    let context = Context::new()?;
    let mut handle = match context.open_device_with_vid_pid(vid, pid) {
        Some(h) => h,
        None => {
            eprintln!("❌ Device {:04x}:{:04x} not found", vid, pid);
            return Ok(());
        }
    };

    println!("✅ Found Maschine Jam device");

    // Find the interrupt endpoint
    let device = handle.device();
    let config = device.active_config_descriptor()?;
    let mut endpoint = None;
    for interface in config.interfaces() {
        if interface.number() == iface {
            for desc in interface.descriptors() {
                for ep in desc.endpoint_descriptors() {
                    if ep.direction() == Direction::In && ep.transfer_type() == TransferType::Interrupt {
                        endpoint = Some(ep.address());
                        break;
                    }
                }
            }
        }
    }
    let endpoint = match endpoint {
        Some(ep) => ep,
        None => {
            eprintln!("❌ No interrupt IN endpoint found on interface {}", iface);
            return Ok(());
        }
    };
    println!("✅ Found endpoint 0x{:02x} on interface {}", endpoint, iface);

    // Detach kernel driver if necessary
    if handle.kernel_driver_active(iface).unwrap_or(false) {
        println!("🔧 Detaching kernel driver...");
        handle.detach_kernel_driver(iface)?;
    }
    handle.claim_interface(iface)?;
    println!("✅ Interface claimed successfully");

    let mut buf = [0u8; 64];
    let timeout = Duration::from_millis(500);
    println!("");
    println!("🎹 Press buttons on your Maschine Jam!");
    println!("📊 You should see USB data below:");
    println!("   (Press Ctrl+C to exit)");
    println!("");

    let mut message_count = 0;
    loop {
        match handle.read_interrupt(endpoint, &mut buf, timeout) {
            Ok(len) if len > 0 => {
                message_count += 1;
                println!("📥 Message #{} ({} bytes): {:02x?}", message_count, len, &buf[..len]);
                
                // Try to interpret the data
                if len >= 17 && buf[0] == 0x01 {
                    println!("   📋 Analysis:");
                    println!("     - Header: 0x{:02x}", buf[0]);
                    println!("     - Data bytes: {:02x?}", &buf[1..len]);
                    
                    // Look for button presses (non-zero bytes)
                    let mut button_presses = Vec::new();
                    for (i, &byte) in buf[1..len].iter().enumerate() {
                        if byte != 0 {
                            button_presses.push((i + 1, byte));
                        }
                    }
                    
                    if !button_presses.is_empty() {
                        println!("     - Button activity detected:");
                        for (pos, value) in button_presses {
                            println!("       Position {}: 0x{:02x} ({})", pos, value, value);
                        }
                    } else {
                        println!("     - No button activity");
                    }
                }
                println!("");
            }
            Ok(_) => {}
            Err(e) => {
                if e == rusb::Error::Timeout {
                    continue;
                } else {
                    eprintln!("❌ Read error: {}", e);
                    break;
                }
            }
        }
    }
    Ok(())
} 