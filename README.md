<p align="center">
  <img src="frontend/static/logo.svg" alt="Sound Blaster G6X" width="120" />
</p>

<h1 align="center">Sound Blaster G6 / G6X Linux Controller</h1>

<p align="center">
  <strong>A native Linux desktop application for controlling Creative Sound Blaster G6 and G6X USB DAC/AMP devices.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/License-GPLv3-blue.svg" alt="License: GPLv3" />
  <img src="https://img.shields.io/badge/Platform-Linux-green.svg" alt="Platform: Linux" />
  <img src="https://img.shields.io/badge/Rust-2024-orange.svg" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/UI-SvelteKit-red.svg" alt="UI: SvelteKit" />
</p>

---

## ✨ Features

- 🎛️ **SBX Audio Profiles** — Toggle and control SBX Surround, Crystalizer, Bass, Smart Volume, and Dialog+
- 🎚️ **10-Band Parametric Equalizer** — Fine-tune your audio with a 10-band EQ and visual frequency response
- 💾 **Custom EQ Presets** — Save, load, and delete your own equalizer presets (stored locally)
- 🎛️ **Mixer Controls** — Full control over playback and capture volumes with real-time visual feedback
- 🔇 **Mute Controls** — Independent mute for speakers and microphone with visual indicators
- 🖥️ **Native Desktop App** — Runs as a native window using WebView (not a browser tab)
- 📌 **System Tray** — Minimize to tray, auto-start on boot
- ⚡ **Lightweight** — Only ~6MB binary, ~2MB packaged (vs 200MB+ for Electron apps)

## 📸 Screenshots

> *Coming soon*

## 📦 Installation

### Debian / Ubuntu / Linux Mint (.deb)
```bash
sudo dpkg -i soundblaster-g6x_2.0.0_amd64.deb
```

### AppImage (Any Distribution)
```bash
chmod +x soundblaster-g6x-2.0.0-x86_64.AppImage
./soundblaster-g6x-2.0.0-x86_64.AppImage
```

To enable autostart with AppImage:
```bash
./soundblaster-g6x-2.0.0-x86_64.AppImage --install-autostart
```

### Flatpak
```bash
flatpak install soundblaster-g6x-2.0.0.flatpak
```

## 🔧 Building from Source

### Prerequisites

```bash
# System dependencies
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libhidapi-dev

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js (for frontend)
sudo apt install nodejs npm
```

### Build

```bash
# Build frontend
cd frontend && npm install && npm run build && cd ..

# Build application (release)
cargo build --release -j 6

# Run
cargo run --release
```

### Build Packages

```bash
# Build all packages (.deb, AppImage, Flatpak)
bash packaging/build-packages.sh all

# Or individually
bash packaging/build-packages.sh deb
bash packaging/build-packages.sh appimage
bash packaging/build-packages.sh flatpak
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│           Native Window (tao + wry)     │
│  ┌───────────────────────────────────┐  │
│  │     WebView (WebKitGTK)           │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │   SvelteKit Frontend (SPA)  │  │  │
│  │  │   - SBX Profile Controls    │  │  │
│  │  │   - 10-Band Equalizer       │  │  │
│  │  │   - Mixer Controls          │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
├─────────────────────────────────────────┤
│         Axum HTTP Server (:3311)        │
│              REST API Layer             │
├─────────────────────────────────────────┤
│     Rust Backend (HID USB Control)      │
│        hidapi → Sound Blaster G6X       │
├─────────────────────────────────────────┤
│  System Tray (tray-icon) + Autostart    │
└─────────────────────────────────────────┘
```

## 🔌 Supported Devices

| Device | Vendor ID | Product ID | Status |
|--------|-----------|------------|--------|
| Sound Blaster G6 | `0x041e` | `0x3256` | ✅ Supported |
| Sound Blaster G6X | `0x041e` | `0x3263` | ✅ Supported |

## 📋 USB HID Access (udev)

To access the Sound Blaster device without `sudo`, add a udev rule:

```bash
sudo tee /etc/udev/rules.d/99-soundblaster-g6x.rules << 'EOF'
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="041e", ATTRS{idProduct}=="3256", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="041e", ATTRS{idProduct}=="3263", MODE="0666"
EOF
sudo udevadm control --reload-rules && sudo udevadm trigger
```

> **Note:** The `.deb` package installs this rule automatically.

## 📜 License

This project is licensed under the **GNU General Public License v3.0** — see the [LICENSE](LICENSE) file for details.

### Acknowledgments

This project is built upon [linuxblaster_control](https://github.com/RizeCrime/linuxblaster_control) by **RizeCrime**, which is licensed under the **MIT License**. The original project provided the foundational USB HID communication layer for Creative Sound Blaster G6/G6X devices on Linux.

See [LICENSE-MIT-ORIGINAL](LICENSE-MIT-ORIGINAL) for the original MIT license.

**Key additions in this fork:**
- Native desktop application (WebView via `wry` + `tao`)
- SvelteKit-based modern UI with dark theme
- 10-band parametric equalizer with custom presets
- System tray integration with autostart
- Mixer controls with mute functionality
- Linux packaging (.deb, AppImage, Flatpak)

## 👨‍💻 Developer

- **GitHub**: [dreamzone-cc](https://github.com/dreamzone-cc)
- **Discord**: `yuuyu_gg`

---

<p align="center">
  <sub>Made with ❤️ for the Linux audio community</sub>
</p>
