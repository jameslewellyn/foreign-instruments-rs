use rusb::{Context, DeviceHandle, UsbContext};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vid = 0x17cc;
    let pid = 0x1500;
    let iface = 0;
    let timeout = Duration::from_millis(500);

    let context = Context::new()?;
    let mut handle = match context.open_device_with_vid_pid(vid, pid) {
        Some(h) => h,
        None => {
            eprintln!("Device {:04x}:{:04x} not found", vid, pid);
            return Ok(());
        }
    };

    // Detach kernel driver if necessary
    if handle.kernel_driver_active(iface).unwrap_or(false) {
        handle.detach_kernel_driver(iface)?;
    }
    handle.claim_interface(iface)?;

    // Find the first interrupt IN endpoint
    let device = handle.device();
    let config = device.active_config_descriptor()?;
    let mut endpoint = None;
    for interface in config.interfaces() {
        if interface.number() == iface {
            for desc in interface.descriptors() {
                for ep in desc.endpoint_descriptors() {
                    if ep.direction() == rusb::Direction::In && ep.transfer_type() == rusb::TransferType::Interrupt {
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
            eprintln!("No interrupt IN endpoint found on interface {}", iface);
            return Ok(());
        }
    };
    println!("Reading from endpoint 0x{:02x} on interface {}", endpoint, iface);

    let mut buf = [0u8; 64];
    println!("Press buttons on Maschine Jam. Ctrl+C to exit.");
    loop {
        match handle.read_interrupt(endpoint, &mut buf, timeout) {
            Ok(len) if len > 0 => {
                println!("Received ({} bytes): {:02x?}", len, &buf[..len]);
            }
            Ok(_) => {},
            Err(e) => {
                if e == rusb::Error::Timeout {
                    continue;
                } else {
                    eprintln!("Read error: {}", e);
                    break;
                }
            }
        }
    }
    Ok(())
} 