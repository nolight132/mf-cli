use rusb::{DeviceHandle, GlobalContext};
use std::{env, fs, path::Path, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID_MF1: u16 = 0xaf80;
const PID_MF2: u16 = 0xaf90;
const PID_MF4: u16 = 0xaf70;
const SYSFS_PATH: &str = "/dev/minifuse_cmd";

fn parse_selector(target: &str) -> Option<u16> {
    match target {
        "inst" => Some(0x0000),
        "48v" => Some(0x0400),
        "monitor" => Some(0x0500),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let pairs_args = &args[1..];

    if pairs_args.is_empty() || pairs_args.len() % 2 != 0 {
        eprintln!("Usage: mf-cli <inst|48v|monitor> <on|off>[<inst|48v|monitor> <on|off> ...]");
        eprintln!("Examples:");
        eprintln!("  mf-cli inst on");
        eprintln!("  mf-cli 48v on monitor off");
        eprintln!("  mf-cli inst on 48v on monitor off");
        std::process::exit(1);
    }

    let mut commands: Vec<(u16, bool, &str)> = Vec::new();
    for chunk in pairs_args.chunks(2) {
        let target = chunk[0].as_str();
        let state = chunk[1].as_str();

        let selector = match parse_selector(target) {
            Some(s) => s,
            None => {
                eprintln!(
                    "Error: Unknown target '{}'. Use 'inst', '48v' or 'monitor'.",
                    target
                );
                std::process::exit(1);
            }
        };

        if state != "on" && state != "off" {
            eprintln!(
                "Error: Unknown state '{}' for target '{}'. Use 'on' or 'off'.",
                state, target
            );
            std::process::exit(1);
        }

        let enable = state == "on";
        commands.push((selector, enable, target));
    }

    // 1. Try using the seamless Kernel Module first
    if Path::new(SYSFS_PATH).exists() {
        println!("[*] Kernel module detected. Sending commands seamlessly...");
        for (selector, enable, target) in &commands {
            let cmd = format!("{:04x} {}", selector, if *enable { 1 } else { 0 });
            match fs::write(SYSFS_PATH, &cmd) {
                Ok(_) => {
                    println!(
                        "[+] {} toggled {}.",
                        target,
                        if *enable { "ON" } else { "OFF" }
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[-] Failed to write command for {} to kernel module: {}",
                        target, e
                    );
                }
            }
            // Small delay to ensure the hardware processes sequential commands
            sleep(Duration::from_millis(50));
        }
        println!("[*] All commands sent via kernel module.");
        return;
    }

    // 2. Fallback to userspace USB manipulation
    println!("[!] Kernel module not found. Falling back to userspace USB (audio may interrupt)...");

    let (mut handle, model) =
        find_minifuse().expect("No MiniFuse device found or permission denied");

    println!("[*] Found {}... applying settings", model);

    for (selector, enable, target) in &commands {
        toggle_feature(&mut handle, *selector, *enable);
        println!(
            "[+] {} toggled {}.",
            target,
            if *enable { "ON" } else { "OFF" }
        );
    }

    let _ = handle.reset();

    println!("[*] All commands sent to {}.", model);
}

fn find_minifuse() -> Option<(DeviceHandle<GlobalContext>, &'static str)> {
    let devices = rusb::devices().ok()?;
    for device in devices.iter() {
        let device_desc = device.device_descriptor().ok()?;
        if device_desc.vendor_id() == VID {
            let model = match device_desc.product_id() {
                PID_MF1 => "MiniFuse 1",
                PID_MF2 => "MiniFuse 2",
                PID_MF4 => "MiniFuse 4",
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
