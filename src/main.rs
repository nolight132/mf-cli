use rusb::UsbContext;
use std::{env, fs, path::PathBuf, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID: u16 = 0xaf80;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("toggle");

    let sysfs_path = find_sysfs_path(VID, PID).expect("[-] MiniFuse 1 not found.");
    let context = rusb::GlobalContext::default();
    let handle = context
        .open_device_with_vid_pid(VID, PID)
        .expect("[-] Open failed.");

    let _ = handle.set_auto_detach_kernel_driver(true);
    let _ = handle.claim_interface(0);

    let enable = match cmd {
        "on" => true,
        "off" => false,
        _ => {
            let mut buf = [0u8; 1];
            let _ =
                handle.read_control(0xA1, 0x01, 0x0400, 0, &mut buf, Duration::from_millis(200));
            buf[0] == 0
        }
    };

    println!(
        "[*] Toggling 48V -> {}...",
        if enable { "ON" } else { "OFF" }
    );

    let data: [u8; 2] = if enable { [1, 0] } else { [0, 0] };
    let _ = handle.write_control(0x21, 34, 0x0400, 0, &data, Duration::from_millis(200));

    drop(handle);

    let _ = fs::write(sysfs_path.join("authorized"), "0");
    sleep(Duration::from_millis(600));
    let _ = fs::write(sysfs_path.join("authorized"), "1");
}

fn find_sysfs_path(vid: u16, pid: u16) -> Option<PathBuf> {
    let vid_s = format!("{:04x}", vid);
    let pid_s = format!("{:04x}", pid);
    fs::read_dir("/sys/bus/usb/devices/")
        .ok()?
        .flatten()
        .find_map(|e| {
            let p = e.path();
            if fs::read_to_string(p.join("idVendor")).ok()?.trim() == vid_s
                && fs::read_to_string(p.join("idProduct")).ok()?.trim() == pid_s
            {
                Some(p)
            } else {
                None
            }
        })
}
