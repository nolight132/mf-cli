use std::{env, fs, path::PathBuf, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID: u16 = 0xaf80;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: mf-control <48v|monitor> <on|off>");
        return;
    }

    let target = args[1].as_str();
    let enable = args[2].as_str() == "on";

    let selector = match target {
        "48v" => 0x0400,
        "monitor" => 0x0500,
        _ => {
            println!("Unknown target");
            return;
        }
    };

    let sysfs = find_sysfs(VID, PID).expect("MiniFuse 1 not found");

    if let Some(handle) = rusb::open_device_with_vid_pid(VID, PID) {
        let _ = handle.set_auto_detach_kernel_driver(true);
        let _ = handle.claim_interface(0);
        let data = if enable { [1, 0] } else { [0, 0] };
        let _ = handle.write_control(0x21, 34, selector, 0, &data, Duration::from_millis(200));
        drop(handle);
    }

    // Reset to apply
    let _ = fs::write(sysfs.join("authorized"), "0");
    sleep(Duration::from_millis(600));
    let _ = fs::write(sysfs.join("authorized"), "1");
    println!(
        "[+] {} toggled {}.",
        target,
        if enable { "ON" } else { "OFF" }
    );
}

fn find_sysfs(vid: u16, pid: u16) -> Option<PathBuf> {
    let v_s = format!("{:04x}", vid);
    let p_s = format!("{:04x}", pid);
    fs::read_dir("/sys/bus/usb/devices/")
        .ok()?
        .flatten()
        .find_map(|e| {
            let p = e.path();
            if fs::read_to_string(p.join("idVendor")).ok()?.trim() == v_s
                && fs::read_to_string(p.join("idProduct")).ok()?.trim() == p_s
            {
                Some(p)
            } else {
                None
            }
        })
}
