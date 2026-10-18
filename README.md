<p align="center">
  <img src="app/src-tauri/icons/128x128.png" width="96" alt="">
</p>

<h1 align="center">PeriHub</h1>

<p align="center">
  An open-source app to configure gaming peripherals on <b>Windows and Linux</b>,<br>
  without SteelSeries GG or Logitech G HUB.
</p>

<p align="center">
  <a href="https://github.com/hex-dump/perihub/releases/latest"><img src="https://img.shields.io/github/v/release/hex-dump/perihub?label=release" alt="Release"></a>
  <a href="https://github.com/hex-dump/perihub/actions/workflows/ci.yml"><img src="https://github.com/hex-dump/perihub/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/hex-dump/perihub" alt="MIT license"></a>
</p>

![Keyboard lighting](docs/screenshots/keyboard-lighting.png)

PeriHub currently supports the **SteelSeries Apex 3 TKL** keyboard and the **Logitech G502 HERO** mouse. The goal is to grow into a single, lightweight, vendor-neutral app for many more devices: see [Hardware roadmap](#hardware-roadmap).

## Contents

- [Features](#features)
- [Supported hardware](#supported-hardware)
- [Hardware roadmap](#hardware-roadmap)
- [Download and install](#download-and-install)
- [Usage](#usage)
- [Safety](#safety)
- [FAQ](#faq)
- [Known limitations](#known-limitations)
- [Building from source](#building-from-source)
- [Architecture](#architecture)
- [Contributing](#contributing)
- [Support the project](#support-the-project)
- [Credits and license](#credits-and-license)

## Features

### SteelSeries Apex 3 TKL

| Feature | Details | Stored in |
|---|---|---|
| 8-zone lighting | Static (one color per zone), gradient, breathing, color cycle, wave, reactive (the zone of each pressed key lights up, Linux only for now), built-in rainbow, off | PC (PeriHub reapplies it whenever the keyboard is connected) |
| Brightness | 0–100% in 17 steps, kept in sync with SteelSeries + F11/F12 | PC |
| Key remapping | Any key, on both the normal layer and the "SteelSeries key held" layer, with the US, UK, Italian, German, French, Spanish, Portuguese, Swedish/Finnish, Norwegian and Danish layouts | **Keyboard memory**: works without PeriHub |

### Logitech G502 HERO

| Feature | Details | Stored in |
|---|---|---|
| Profiles | 5 onboard profiles: activate, enable/disable, restore factory settings | **Mouse memory** |
| DPI | Up to 5 levels from 100 to 25600 (step 50), default and sniper level, live preview | Mouse memory |
| Polling rate | 125, 250, 500, 1000 Hz | Mouse memory |
| Buttons | 11 buttons plus the G-Shift layer: mouse clicks, keys with Ctrl/Shift/Alt/Win/Alt Gr, media keys, DPI up/down/cycle/sniper, profile switching | Mouse memory |
| Lighting | Logo and DPI indicator: off, fixed color with brightness, breathing, color cycle (breathing and color cycle stay in sync on both zones), with live preview | Mouse memory |

### App profiles

While a game or an app is in focus, PeriHub can show one of your saved keyboard lightings and switch the mouse to one of its onboard profiles. When you switch to an app without a rule, both go back to how they were. It works on Windows and, on Linux, on Hyprland: other Linux desktops do not tell apps which window is in focus.

PeriHub only reads the app's identifier (the executable name on Windows, the window class on Linux), never window titles or contents, and keeps nothing but your rules.

### Other Logitech G mice (experimental)

27 more Logitech mice are recognized when connected with a USB cable (see [Supported hardware](#supported-hardware)). They have not been tested with PeriHub yet, so it only reads them: you can see their profiles, DPI levels, buttons and lighting, but nothing can be changed until someone confirms that the model works.

<table>
  <tr>
    <td><img src="docs/screenshots/keyboard-keys.png" alt="Key remapping"></td>
    <td><img src="docs/screenshots/mouse-dpi.png" alt="Mouse DPI and polling rate"></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/mouse-buttons.png" alt="Mouse buttons"></td>
    <td><img src="docs/screenshots/mouse-lighting.png" alt="Mouse lighting"></td>
  </tr>
</table>

## Supported hardware

| Device | USB ID | Status |
|---|---|---|
| SteelSeries Apex 3 TKL | `1038:1622` (firmware 3.0.4) | ✅ Supported, tested on real hardware |
| Logitech G502 HERO | `046d:c08b` | ✅ Supported, tested on real hardware |
| Other Logitech G mice, over a USB cable | listed below | 🧪 Experimental, read-only |

<details>
<summary>Experimental Logitech mice</summary>

The list comes from [libratbag](https://github.com/libratbag/libratbag). Wireless models are recognized only when connected with their USB cable: LIGHTSPEED receivers are not supported yet.

| Mouse | USB ID |
|---|---|
| Logitech G502 Proteus Core | `046d:c07d` |
| Logitech G502 Proteus Spectrum | `046d:c332` |
| Logitech G502 HERO Wireless | `046d:c08d` |
| Logitech G502 X | `046d:c099` |
| Logitech G502 X Wireless | `046d:c098` |
| Logitech G502 X PLUS | `046d:c095` |
| Logitech G Pro | `046d:c085` |
| Logitech G Pro | `046d:c08c` |
| Logitech G Pro Wireless | `046d:c088` |
| Logitech G Pro X Superlight | `046d:c094` |
| Logitech G102/G103/G203 | `046d:c084` |
| Logitech G102/G103/G203 | `046d:c092` |
| Logitech G102/G103/G203 | `046d:c09d` |
| Logitech G302 | `046d:c07f` |
| Logitech G303 | `046d:c080` |
| Logitech G303 Shroud Edition | `046d:c097` |
| Logitech G402 | `046d:c07e` |
| Logitech G403 | `046d:c083` |
| Logitech G403 HERO | `046d:c08f` |
| Logitech G403 Wireless | `046d:c082` |
| Logitech G703 | `046d:c087` |
| Logitech G703 HERO | `046d:c090` |
| Logitech G705 | `046d:c096` |
| Logitech G900 | `046d:c081` |
| Logitech G903 | `046d:c086` |
| Logitech G903 HERO | `046d:c091` |
| Logitech MX518 | `046d:c08e` |

If you own one of these, please fill in a [mouse test report](https://github.com/hex-dump/perihub/issues/new?template=mouse_test.yml) with what PeriHub shows: it takes a few minutes, and a model becomes fully supported once someone confirms it works.

</details>

PeriHub only talks to the devices listed here. It never sends commands to anything it does not recognize.

## Hardware roadmap

PeriHub is meant to support much more than these two devices. The plan is to add hardware over time, starting with devices that share protocols PeriHub already speaks:

- **Logitech G mice using HID++ 2.0.** 27 more models are already recognized in read-only mode. Next: enable editing as owners confirm them, and support LIGHTSPEED wireless receivers.
- **Other SteelSeries keyboards**, such as the rest of the Apex range.
- **SteelSeries mice.**
- **Headsets and other peripherals**, where the hardware exposes useful settings.
- **More languages** for the UI, and more layouts for the key remapping view.

There is no fixed timeline: devices are added once their protocol can be verified on real hardware. If you own a device you would like to see supported, open a [device request](https://github.com/hex-dump/perihub/issues/new?template=device_request.yml): read-only dumps and testing help are the fastest way to get a device added.

## Download and install

Get the file for your system from the [releases page](https://github.com/hex-dump/perihub/releases/latest).

### Windows

1. Download `PeriHub_…_x64-setup.exe` (or the `.msi`) and run it.
2. The installer is not code-signed. If SmartScreen blocks it, click **More info → Run anyway**.
3. **Quit SteelSeries GG and Logitech G HUB** and turn off their autostart: two programs talking to the same device get in each other's way.

No drivers are needed: PeriHub uses the standard Windows HID interfaces.

### Linux

**AppImage** (any distribution, including Arch and CachyOS):

```sh
chmod +x PeriHub_*_amd64.AppImage
./PeriHub_*_amd64.AppImage
```

**Arch, CachyOS and other Arch-based distributions**: the `perihub-bin` package will be published on the AUR soon. Until then, you can build it from the repository; it installs the udev rule too:

```sh
git clone https://github.com/hex-dump/perihub.git
cd perihub/packaging/aur/perihub-bin
makepkg -si
```

**Debian/Ubuntu** (`.deb`) and **Fedora/openSUSE** (`.rpm`): install the package with your distribution's package manager.

#### USB permissions

The `.deb` and `.rpm` packages install a udev rule automatically. With the AppImage, if PeriHub shows "permission denied", install the rule manually:

```sh
sudo curl -o /etc/udev/rules.d/70-perihub.rules \
  https://raw.githubusercontent.com/hex-dump/perihub/main/packaging/linux/70-perihub.rules
sudo udevadm control --reload && sudo udevadm trigger
```

The rule grants access only to the HID interfaces of the supported devices, and only to the user with the active session. On the Apex 3 TKL it opens only the configuration interface: the interfaces that report what you type stay closed. If you already use OpenRGB with its udev rules, you probably don't need it.

#### Optional: reactive lighting

Reactive lighting needs a second rule, [`71-perihub-keys.rules`](packaging/linux/71-perihub-keys.rules), because it reads key presses. PeriHub offers to install it from **Lighting → Reactive**, asking for your password, and can remove it from the same place.

> [!WARNING]
> Linux cannot give this permission to PeriHub alone. With the rule installed, **every program running under your user account can read everything you type on the Apex 3 TKL**, including passwords and private messages, in any app and even while PeriHub is closed. A malicious or compromised program could use it as a keylogger. It turns off, for this keyboard, the protection Wayland normally gives. Install it only if you want reactive lighting and trust all the software you run.

PeriHub itself reads key presses only while the reactive effect is on and keeps only which lighting zone to light: it never stores, logs or sends them.

## Usage

- **Keyboard → Lighting**: pick an effect. Changes apply immediately and are remembered.
- **Keyboard → Keys**: choose your layout (ANSI US, ISO UK, ISO Italian), click a key, pick its new function and press **Save to keyboard**. Switch to **With SteelSeries key** to change what keys do while the SteelSeries key is held.
- **Mouse**: pick a profile at the top, then edit DPI, buttons and lighting. Nothing is written to the mouse until you press **Save to mouse**. DPI and LED changes are previewed live on the mouse before saving; leaving the page restores the saved settings.
- **G-Shift**: assign "G-Shift" to a button. While you hold it, the other buttons use the functions from the G-Shift tab.

### Running in the background

Closing the window does **not** quit PeriHub. It keeps running to animate the keyboard lighting and to reapply it when the keyboard is plugged back in or the PC wakes up. Reopen it from the tray icon or by launching it again. To quit for real: **Settings → Quit PeriHub**.

**Autostart** can be enabled in Settings. On Hyprland, XDG autostart entries only run when `xdg-desktop-autostart.target` is active (uwsm or systemd). Otherwise, add this to your configuration:

```
exec-once = /path/to/perihub --hidden
```

To see the tray icon on Hyprland or Sway you need a bar with a tray module, such as Waybar or the Noctalia bar.

## Safety

PeriHub does not modify firmware. It uses the same configuration commands as the official apps, with extra safeguards:

- **Restricted commands.** The code can only send commands that have been verified.
  - The command that reboots the Apex into its bootloader cannot even be expressed.
  - On the G502, only HID++ features `0x0000`, `0x0001`, `0x0003`, `0x0005`, `0x2201`, `0x8060`, `0x8070` and `0x8100` are used.
- **Verified writes.**
  - Mouse: profiles are edited on top of the original sector bytes, so fields PeriHub does not manage stay intact. Every written sector is read back and compared byte by byte.
  - Keyboard: the key mapping is written to RAM first and only saved to memory if the read-back matches; otherwise it is rolled back.
- **Untested mice are read-only.** For a mouse that has not been verified, the core only allows HID++ functions that read state, and it does not even take the mouse out of software mode.
- **Factory profiles are read-only.** The mouse's ROM profiles are never written and are used for restoring.
- **Key presses stay on your PC.** Only the reactive lighting effect reads key presses, only while it is on, and only from the Apex 3 TKL. PeriHub keeps nothing but the time of the last press in each lighting zone: which keys you press is never stored, logged or sent anywhere. On Linux, the interfaces that report key presses are closed by default and opened only if you install the optional rule for reactive lighting (see [the warning](#optional-reactive-lighting)).
- **Backups.** Before every write to a device's memory, PeriHub saves the previous state in
  - Linux: `~/.config/perihub/backups/`
  - Windows: `%APPDATA%\perihub\backups\`

To report a security issue, see [SECURITY.md](SECURITY.md).

## FAQ

**Can I keep GG or G HUB running?**
No, quit them. On Windows, G HUB in particular takes control of the mouse and would undo your changes.

**Does this void my warranty?**
PeriHub does not touch the firmware. It only writes settings (profiles, key mapping) that the official apps write too.

**Why does the keyboard go back to the rainbow when PeriHub is closed?**
The Apex 3 TKL does not store colors, and it behaves the same way with GG. PeriHub reapplies them as soon as it detects the keyboard. "Built-in rainbow" is the only effect generated by the keyboard itself.

**How do I go back to factory settings?**
Keyboard: **Keys → Reset all to factory**. Mouse: **Factory settings** on each profile. Backups are kept as well.

**Is the UI available in other languages?**
The UI is in English. More languages are on the roadmap.

## Known limitations

- **Windows**: first test on a real Windows PC passed (keyboard lighting, key remapping, mouse profiles). Reports from other setups are welcome.
- **Logitech wireless mice** work only over their USB cable: LIGHTSPEED receivers are not supported yet.
- **App profiles** detect the app in focus on Windows and on Hyprland only. They switch lighting and mouse profiles, not key mappings, because changing those writes the keyboard's memory.
- **Not yet available**:
  - software macros;
  - reactive lighting on Windows (it works on Linux);
  - GameSense compatibility.
- **Out of scope**: SteelSeries Sonar and Moments, G HUB Lua scripts.

## Building from source

You need stable [Rust](https://rustup.rs) (≥ 1.88) and [Node.js](https://nodejs.org) ≥ 20.19. On Linux you also need the development libraries for WebKitGTK 4.1, libayatana-appindicator, librsvg and libudev:

```sh
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libudev-dev
# Arch/CachyOS
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator librsvg
```

```sh
git clone https://github.com/hex-dump/perihub.git
cd perihub/app
npm install
npm run tauri dev                     # development, with hot reload
npm run tauri build -- --no-bundle    # standalone binary in target/release/perihub
npm run tauri build                   # installers for the current platform
```

Don't build the app with plain `cargo build`: that binary loads the UI from the development server (`localhost:1420`).

Tests and checks:

```sh
cargo fmt --all                 # formatting (CI checks it)
cargo test --workspace          # includes tests on sectors read from a real G502 HERO
cargo clippy --workspace --all-targets
cd app && npm run check         # frontend type check
```

### Command-line tool

Useful for diagnostics and scripting. Don't run it while the app is open, since both would compete for the same devices.

```sh
cargo run -p perihub-cli -- status
cargo run -p perihub-cli -- mouse state
cargo run -p perihub-cli -- keyboard keymap
cargo run -p perihub-cli -- keyboard light '{"type":"wave","period_ms":3000,"reverse":false}' 10
```

## Architecture

```
crates/perihub-core     Rust library: protocols, effects, device detection, configuration
  src/apex/             SteelSeries Apex 3 TKL: RGB (output reports) and key mapping (feature reports)
  src/hidpp/            Logitech HID++ 2.0, the onboard profile format (CRC-CCITT) and the list of mice
  src/effects.rs        lighting effects rendered on the PC
  src/manager.rs        device hotplug
  src/worker.rs         one thread per device, so requests never interleave
  tests/fixtures/       real G502 HERO sectors used by the tests
crates/perihub-cli      command-line tool
app/                    UI: Tauri 2 + Svelte 5
packaging/linux         udev rule
```

- Device communication goes through [hidapi](https://github.com/libusb/hidapi). On Linux the hidraw backend is used, so the keyboard and mouse keep working normally while PeriHub is open.
- On Windows the mouse's short and long HID++ collections are separate devices; PeriHub uses the long one (report `0x11`).
- Each device family is a separate module in `perihub-core`, which is where new hardware will be added.

## Contributing

Bug reports, device requests and pull requests are welcome.

- **Questions and ideas**: use [Discussions](https://github.com/hex-dump/perihub/discussions).
- **Bugs**: include your OS, the device, and what the app showed.
- **New devices**: open a [device request](https://github.com/hex-dump/perihub/issues/new?template=device_request.yml) with the model and USB ID.
- **Experimental Logitech mice**: fill in a [mouse test report](https://github.com/hex-dump/perihub/issues/new?template=mouse_test.yml).
- **Code**: run `cargo fmt`, `cargo test`, `cargo clippy` and `npm run check` before opening a pull request.

## Support the project

PeriHub is free and open source. If it spared you from installing G HUB or SteelSeries GG and you'd like to say thanks, you can [buy me a coffee](https://buymeacoffee.com/hexdump1).

Donations are entirely optional and don't unlock anything. They support the development of PeriHub, including buying new devices to add. You can help just as much without money: test PeriHub on your hardware, report bugs, or open a device request with read-only dumps.

## Credits and license

The protocols were reconstructed thanks to the work of:

- [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB): documentation of the 8-zone Apex lighting protocol. No code was copied.
- [libratbag](https://github.com/libratbag/libratbag): Logitech HID++ 2.0, onboard profiles and the list of Logitech mice (MIT).
- [gorkemgun/apex-3-tkl](https://github.com/gorkemgun/apex-3-tkl): Apex 3 TKL key mapping (MIT).

PeriHub is released under the [MIT license](LICENSE). It is not affiliated with SteelSeries or Logitech; product names belong to their respective owners.
