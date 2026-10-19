# Changelog

## 0.3.0 — 2026-10-19

The new features were tested on Linux with the real hardware. On Windows they have not been tried on a real PC yet: reports are welcome in issue #3.

### Added
- App profiles: while a game or an app is in focus, the keyboard shows a saved lighting and the mouse switches to one of its onboard profiles; both go back when it loses focus. Works on Windows and on Hyprland.
- Saved lightings on the keyboard lighting page, used by app profiles.
- Reactive keyboard lighting: the zone of each pressed key lights up and fades back. On Windows the keys come from Raw Input (not tested on a real PC yet). Key presses are read only while the effect is on and are never stored. It needs an optional udev rule, which PeriHub can install and remove after an explicit warning: with it, every program running as the user can read what is typed on the Apex.
- `perihub-cli focus` prints the app in focus, to find the name to use in a rule.
- German, French, Spanish, Portuguese, Swedish/Finnish, Norwegian and Danish layouts in the key remapping view, chosen automatically from the system language.
- Mouse macros stored in the mouse's memory, so they work without PeriHub on any PC. Record them from the keyboard or build them step by step with keys, mouse clicks, media keys, scrolling and pauses. A macro runs once, repeats while the button is held, or runs one part when the button is pressed and another when it is released. Macros are saved on the G502 HERO; other models show theirs read-only.
- Updates on Windows: PeriHub looks for a new version on GitHub when it starts and installs it with one click, after checking its signature. The check can be turned off in Settings, which also shows the version. Install 0.3.0 by hand: updates start from the next version.

### Changed
- Mouse backups include every sector of the memory, macros too.
- **Security:** the Linux udev rule now opens only the Apex 3 TKL's configuration interface. Before, it also opened the interfaces that report key presses, so programs running as the user could read what was typed on it. Install the new rule to apply the change.
- The experimental mouse notice points to the new mouse test report form on GitHub.

## 0.2.0 — 2026-10-18

First version tried on a real Windows PC: keyboard lighting, key remapping and mouse profiles work.

### Added
- Experimental, read-only support for 27 more Logitech G mice connected with a USB cable, from libratbag's device list. PeriHub shows their profiles, DPI, buttons and lighting but changes nothing until a model is confirmed to work.
- The Linux udev rule covers the new mice.
- Brightness for the mouse's fixed colors.

### Changed
- The mouse page, sidebar and settings show the connected model instead of assuming the G502 HERO.
- A mouse that fails to open is no longer retried every 2 seconds unless it is a tested model; plug it in again to retry.
- Mouse breathing and color cycle apply to all LED zones at once, so the DPI indicator and the logo stay in sync.

### Fixed
- The keyboard brightness slider now follows SteelSeries + F11/F12 while the lighting page is open, instead of only when the page is opened. The new level is also saved, so it is kept when the keyboard is plugged in again.
- The brightness slider moves smoothly, both when dragged and when the level changes from the keyboard.
- The key remapping view matches the real Apex 3 TKL: the bottom row ends with Alt, Win, SteelSeries and Ctrl (there is no Menu key), and there are no Print Screen, Scroll Lock and Pause keys.
- SteelSeries + F10 is labeled as the microphone mute key; SteelSeries + F9, which does nothing, shows no label.

## 0.1.0 — 2026-10-17

First release.

### SteelSeries Apex 3 TKL
- 8-zone lighting: static, gradient, breathing, color cycle, wave, built-in rainbow, off.
- Brightness 0–16, kept in sync with SteelSeries + F11/F12.
- Key remapping on the normal and SteelSeries layers, stored in the keyboard's memory.
- ANSI (US), ISO (UK) and ISO (Italian) layouts in the key remapping view.
- Reset all keys to factory settings.

### Logitech G502 HERO
- 5 onboard profiles: activate, enable/disable, restore factory settings.
- Up to 5 DPI levels (100–25600), default and sniper level, with live preview.
- Polling rate 125/250/500/1000 Hz.
- 11 buttons plus the G-Shift layer: clicks, keys with modifiers, media keys, DPI and profile functions.
- Logo and DPI indicator LEDs: off, fixed, breathing, color cycle, with live preview.

### App
- Automatic detection when devices are plugged in or unplugged.
- System tray icon, autostart on login, single instance.
- Automatic backup before every write to a device's memory.

### Known limitations
- The Windows build is produced and tested by CI but has not been tried on a real Windows PC yet.
- Not yet available: per-game profile switching, software macros, reactive lighting, GameSense.
