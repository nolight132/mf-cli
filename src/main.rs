use rusb::{DeviceHandle, GlobalContext};
use std::{env, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID_MF1: u16 = 0xaf80;
const PID_MF2: u16 = 0xaf90;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: mf-control <inst|48v|monitor> <on|off>");
        std::process::exit(1);
    }

    let target = args[1].as_str();
    let enable = args[2].as_str() == "on";

    let selector = match target {
        "inst" => 0x0000,
        "48v" => 0x0400,
        "monitor" => 0x0500,
        _ => {
            eprintln!(
                "Error: Unknown target '{}'. Use 'inst', '48v' or 'monitor'.",
                target
            );
            std::process::exit(1);
        }
    };

    let (mut handle, model) =
        find_minifuse().expect("No MiniFuse device found or permission denied");

    println!("[*] Found {}... applying settings", model);

    toggle_feature(&mut handle, selector, enable);

    let _ = handle.reset();

    println!(
        "[+] {} toggled {}. Command sent to {}.",
        target,
        if enable { "ON" } else { "OFF" },
        model
    );
}

fn find_minifuse() -> Option<(DeviceHandle<GlobalContext>, &'static str)> {
    let devices = rusb::devices().ok()?;
    for device in devices.iter() {
        let device_desc = device.device_descriptor().ok()?;
        if device_desc.vendor_id() == VID {
            let model = match device_desc.product_id() {
                PID_MF1 => "MiniFuse 1",
                PID_MF2 => "MiniFuse 2",
                _ => continue,
            };
            if let Ok(handle) = device.open() {
                return Some((handle, model));
            }
        }
    }
    None
}

fn toggle_feature(handle: &mut DeviceHandle<GlobalContext>, selector: u16, enable: bool) {
    let _ = handle.set_auto_detach_kernel_driver(true);

    if let Err(e) = handle.claim_interface(0) {
        eprintln!("Warning: Could not claim interface: {}", e);
    }

    let data = if enable { [1, 0] } else { [0, 0] };

    // Control transfer
    handle
        .write_control(0x21, 34, selector, 0, &data, Duration::from_millis(200))
        .expect("Failed to send control command");

    sleep(Duration::from_millis(100));
    let _ = handle.release_interface(0);
}
