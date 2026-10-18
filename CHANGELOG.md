# Changelog

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
- SteelSeries + F9 and F10 are labeled: F9 has no effect, F10 is the microphone mute key.

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
