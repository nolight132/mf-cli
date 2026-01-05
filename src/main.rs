use rusb::{DeviceHandle, GlobalContext};
use std::{env, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID: u16 = 0xaf80;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: mf-control <inst|48v|monitor> <on|off>");
        std::process::exit(1);
    }

    let target = args[1].as_str();
    let enable = args[2].as_str() == "on";

    let selector = match target {
        "inst" => 0,
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

    let mut handle = rusb::open_device_with_vid_pid(VID, PID)
        .expect("MiniFuse 1 not found or permission denied");

    toggle_feature(&mut handle, selector, enable);

    handle.reset().expect("Failed to reset device");

    println!(
        "[+] {} toggled {}. Device reset triggered.",
        target,
        if enable { "ON" } else { "OFF" }
    );
}

fn toggle_feature(handle: &mut DeviceHandle<GlobalContext>, selector: u16, enable: bool) {
    let _ = handle.set_auto_detach_kernel_driver(true);
    let _ = handle.claim_interface(0);

    let data = if enable { [1, 0] } else { [0, 0] };

    // Control transfer
    handle
        .write_control(0x21, 34, selector, 0, &data, Duration::from_millis(200))
        .expect("Failed to send control command");

    sleep(Duration::from_millis(100));
}
