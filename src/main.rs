use clap::{CommandFactory, Parser, error::ErrorKind};
use rusb::{DeviceHandle, GlobalContext};
use std::{fs, path::Path, process, thread::sleep, time::Duration};

const VID: u16 = 0x1c75;
const PID_MF1: u16 = 0xaf80;
const PID_MF1_MSD: u16 = 0xaf82;
const PID_MF2: u16 = 0xaf90;
const PID_MF4: u16 = 0xaf70;
const SYSFS_PATH: &str = "/dev/minifuse_cmd";

const AFTER_HELP: &str = "\
Targets:
  MiniFuse 1    inst, 48v, monitor
  MiniFuse 2    inst/inst1, inst2, 48v, monitor
  MiniFuse 4    inst1 (or inst), inst2, 48v

Examples:
  mf-cli inst on
  mf-cli inst1 on inst2 on
  mf-cli 48v on monitor off
  mf-cli inst1 on inst2 off 48v on";

/// Toggle hardware settings on an Arturia MiniFuse.
#[derive(Parser)]
#[command(version, after_help = AFTER_HELP)]
struct Cli {
    /// Pairs of a target and its state (on or off)
    #[arg(required = true, num_args = 2.., value_names = ["TARGET", "STATE"])]
    settings: Vec<String>,
}

#[derive(Clone, Copy)]
enum Model {
    MiniFuse1,
    MiniFuse2,
    MiniFuse4,
}

impl Model {
    fn from_pid(pid: u16) -> Option<Model> {
        match pid {
            PID_MF1 | PID_MF1_MSD => Some(Model::MiniFuse1),
            PID_MF2 => Some(Model::MiniFuse2),
            PID_MF4 => Some(Model::MiniFuse4),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Model::MiniFuse1 => "MiniFuse 1",
            Model::MiniFuse2 => "MiniFuse 2",
            Model::MiniFuse4 => "MiniFuse 4",
        }
    }

    fn targets(self) -> &'static str {
        match self {
            Model::MiniFuse1 => "inst, 48v or monitor",
            Model::MiniFuse2 => "inst/inst1, inst2, 48v or monitor",
            Model::MiniFuse4 => "inst1, inst2 or 48v",
        }
    }
}

/// One requested change, already validated against the connected model.
struct Setting<'a> {
    target: &'a str,
    selector: u16,
    enable: bool,
}

/// Maps a target name to the control transfer's wValue, which is
/// `(feature << 8) | channel`. Returns `None` when the model lacks the target.
fn parse_selector(model: Model, target: &str) -> Option<u16> {
    match model {
        Model::MiniFuse4 => match target {
            "inst" | "inst1" => Some(0x0000),
            "inst2" => Some(0x0001),
            "48v" => Some(0x0300),
            _ => None,
        },
        // MiniFuse 1 has one instrument input; MiniFuse 2 has two (same USB protocol).
        Model::MiniFuse1 | Model::MiniFuse2 => match target {
            "inst" | "inst1" => Some(0x0000),
            "inst2" if matches!(model, Model::MiniFuse2) => Some(0x0001),
            "48v" => Some(0x0400),
            "monitor" => Some(0x0500),
            _ => None,
        },
    }
}

fn parse_state(state: &str) -> Option<bool> {
    match state {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

/// Prints `message` as an error and exits with status 1.
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}

fn main() {
    let cli = Cli::parse();
    let mut cmd = Cli::command();

    let pairs = cli.settings.chunks_exact(2);
    if !pairs.remainder().is_empty() {
        cmd.error(
            ErrorKind::WrongNumberOfValues,
            "every target needs a state (on or off)",
        )
        .exit();
    }

    let Some(model) = detect_model() else {
        fail("no MiniFuse found");
    };

    let mut settings = Vec::new();
    for pair in pairs {
        let (target, state) = (pair[0].as_str(), pair[1].as_str());
        let Some(selector) = parse_selector(model, target) else {
            cmd.error(
                ErrorKind::InvalidValue,
                format!(
                    "{} has no target '{target}', use {}",
                    model.name(),
                    model.targets()
                ),
            )
            .exit();
        };
        let Some(enable) = parse_state(state) else {
            cmd.error(
                ErrorKind::InvalidValue,
                format!("invalid state '{state}' for {target}, use on or off"),
            )
            .exit();
        };
        settings.push(Setting {
            target,
            selector,
            enable,
        });
    }

    if Path::new(SYSFS_PATH).exists() {
        send_via_kernel_module(&settings);
    } else {
        eprintln!("warning: kernel module not loaded, using userspace USB (audio may drop out)");
        send_via_usb(&settings);
    }

    let summary: Vec<String> = settings
        .iter()
        .map(|s| format!("{} {}", s.target, if s.enable { "on" } else { "off" }))
        .collect();
    println!("{}: {}", model.name(), summary.join(", "));
}

/// Sends each setting through the minifuse_mod device node, which leaves the
/// audio stream running. Exits on the first write the module rejects.
fn send_via_kernel_module(settings: &[Setting]) {
    for setting in settings {
        let cmd = format!("{:04x} {}", setting.selector, setting.enable as u8);
        if let Err(e) = fs::write(SYSFS_PATH, &cmd) {
            fail(format!("kernel module rejected {}: {e}", setting.target));
        }
        // Give the device time to process each command before the next one.
        sleep(Duration::from_millis(50));
    }
}

/// Sends each setting with libusb. Claiming the interface detaches the audio
/// driver, so playback stops until the device reset at the end.
fn send_via_usb(settings: &[Setting]) {
    let mut handle = find_minifuse().unwrap_or_else(|e| fail(format!("cannot open MiniFuse: {e}")));

    for setting in settings {
        if let Err(e) = toggle_feature(&mut handle, setting.selector, setting.enable) {
            fail(format!("failed to set {}: {e}", setting.target));
        }
    }

    let _ = handle.reset();
}

fn detect_model() -> Option<Model> {
    let devices = rusb::devices().ok()?;
    for device in devices.iter() {
        let device_desc = device.device_descriptor().ok()?;
        if device_desc.vendor_id() != VID {
            continue;
        }
        if let Some(model) = Model::from_pid(device_desc.product_id()) {
            return Some(model);
        }
    }
    None
}

/// Opens the first supported MiniFuse. Returns the last open error, such as a
/// permission error when the udev rule is missing, or `NoDevice` if none is
/// connected.
fn find_minifuse() -> Result<DeviceHandle<GlobalContext>, rusb::Error> {
    let mut error = rusb::Error::NoDevice;
    for device in rusb::devices()?.iter() {
        let device_desc = device.device_descriptor()?;
        if device_desc.vendor_id() == VID && Model::from_pid(device_desc.product_id()).is_some() {
            match device.open() {
                Ok(handle) => return Ok(handle),
                Err(e) => error = e,
            }
        }
    }
    Err(error)
}

fn toggle_feature(
    handle: &mut DeviceHandle<GlobalContext>,
    selector: u16,
    enable: bool,
) -> Result<(), rusb::Error> {
    let _ = handle.set_auto_detach_kernel_driver(true);

    if let Err(e) = handle.claim_interface(0) {
        eprintln!("warning: could not claim interface 0: {e}");
    }

    let data = [enable as u8, 0];
    let result = handle.write_control(0x21, 34, selector, 0, &data, Duration::from_millis(200));

    sleep(Duration::from_millis(100));
    let _ = handle.release_interface(0);
    result.map(|_| ())
}
