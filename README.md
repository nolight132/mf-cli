# mf-cli

`mf-cli` is a lightweight Linux command-line utility written in Rust for controlling hardware features of the Arturia MiniFuse 1/2 audio interface. It allows you to toggle hardware settings that are normally only accessible via the proprietary "MiniFuse Control Center" software on Windows or macOS.

## Key Features
- Phantom Power (+48V)
- Direct Mono
- Instrument Mode (INST)
- Non-Sudo Operation

## Installation
### Arch Linux (AUR)
Install using your favorite AUR helper:
```bash
yay -S mf-cli
```

### Manual build
Ensure you have `cargo`, `libusb`, and `systemd-libs` installed.
```bash
git clone https://github.com/nolight132/mf-cli
cd mf-cli
cargo build --release
sudo cp target/release/mf-cli /usr/bin/
```

## Configuration (Permissions)
To control the device without `sudo`, you must install the provided udev rule. The AUR package does this automatically. If installing manually:

1. Copy `99-minifuse.rules` to `/etc/udev/rules.d/`.

2. Reload rules:

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Usage
The syntax is straightforward: mf-cli <target> <on|off>
### **Examples:**
#### Toggle Phantom +48V Power
```bash
mf-cli 48v on
mf-cli 48v off
```

#### Toggle Direct Monitoring Mono:

```bash
mf-cli monitor on
mf-cli monitor off
```
#### Toggle Instrument Mode:

```bash
mf-cli inst on
mf-cli inst off
```

## License
This project is licensed under the MIT License.
