#!/bin/bash
set -e

# ============================================
# Sound Blaster G6X Controller - Package Builder
# Builds: .deb, AppImage, Flatpak
# ============================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PKG_DIR="$SCRIPT_DIR"
OUTPUT_DIR="$PROJECT_DIR/dist"
BINARY_NAME="linuxblaster_control"
APP_NAME="soundblaster-g6x"
APP_LABEL="Sound Blaster G6X Controller"
VERSION="2.0.1"
ARCH="amd64"

BINARY="$PROJECT_DIR/target/release/$BINARY_NAME"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

info() { echo -e "${CYAN}[INFO]${NC} $1"; }
ok()   { echo -e "${GREEN}[OK]${NC} $1"; }
err()  { echo -e "${RED}[ERROR]${NC} $1"; }

# Check binary exists
if [ ! -f "$BINARY" ]; then
    err "Release binary not found at $BINARY"
    info "Building release binary (6 cores)..."
    cd "$PROJECT_DIR"
    CARGO_BUILD_JOBS=6 cargo build --release -j 6
fi

mkdir -p "$OUTPUT_DIR"
BINARY_SIZE=$(du -sh "$BINARY" | cut -f1)
info "Binary: $BINARY ($BINARY_SIZE)"

# ============================================
# 1. BUILD .DEB PACKAGE
# ============================================
build_deb() {
    info "Building .deb package..."

    DEB_DIR="$OUTPUT_DIR/deb-staging"
    rm -rf "$DEB_DIR"

    # Directory structure
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/usr/share/applications"
    mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$DEB_DIR/etc/udev/rules.d"
    mkdir -p "$DEB_DIR/etc/xdg/autostart"

    # Control file
    cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: $APP_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: dreamzone-cc <dreamzone@github.com>
Description: Linux controller for Creative Sound Blaster G6/G6X USB DAC
 A native desktop application for controlling Creative Sound Blaster
 G6 and G6X USB DAC/AMP devices on Linux. Features include SBX audio
 profiles, 10-band parametric equalizer, mixer controls, and system
 tray integration.
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libayatana-appindicator3-1 | libappindicator3-1
Section: sound
Priority: optional
Homepage: https://github.com/dreamzone-cc
EOF

    # Post-install script (udev rules reload)
    cat > "$DEB_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/bash
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger 2>/dev/null || true
EOF
    chmod 755 "$DEB_DIR/DEBIAN/postinst"

    # Copy binary
    cp "$BINARY" "$DEB_DIR/usr/bin/$APP_NAME"
    chmod 755 "$DEB_DIR/usr/bin/$APP_NAME"

    # Copy desktop file
    cp "$PKG_DIR/$APP_NAME.desktop" "$DEB_DIR/usr/share/applications/"

    # Copy icon
    cp "$PKG_DIR/$APP_NAME.png" "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/"

    # Udev rule for Sound Blaster G6/G6X HID access
    cat > "$DEB_DIR/etc/udev/rules.d/99-soundblaster-g6x.rules" << 'EOF'
# Creative Sound Blaster G6/G6X - Allow user access to HID device
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="041e", ATTRS{idProduct}=="3256", MODE="0666"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="041e", ATTRS{idProduct}=="3263", MODE="0666"
EOF

    # Autostart entry (starts minimized in tray on login)
    cp "$PKG_DIR/$APP_NAME-autostart.desktop" "$DEB_DIR/etc/xdg/autostart/$APP_NAME.desktop"

    # Build deb
    DEB_FILE="$OUTPUT_DIR/${APP_NAME}_${VERSION}_${ARCH}.deb"
    dpkg-deb --build "$DEB_DIR" "$DEB_FILE" 2>&1
    rm -rf "$DEB_DIR"

    ok ".deb package: $DEB_FILE ($(du -sh "$DEB_FILE" | cut -f1))"
}

# ============================================
# 2. BUILD AppImage
# ============================================
build_appimage() {
    info "Building AppImage..."

    APPDIR="$OUTPUT_DIR/AppDir"
    rm -rf "$APPDIR"

    # AppDir structure
    mkdir -p "$APPDIR/usr/bin"
    mkdir -p "$APPDIR/usr/share/applications"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

    # Copy binary
    cp "$BINARY" "$APPDIR/usr/bin/$APP_NAME"
    chmod 755 "$APPDIR/usr/bin/$APP_NAME"

    # Copy desktop + icon
    cp "$PKG_DIR/$APP_NAME.desktop" "$APPDIR/"
    cp "$PKG_DIR/$APP_NAME.desktop" "$APPDIR/usr/share/applications/"
    cp "$PKG_DIR/$APP_NAME.png" "$APPDIR/"
    cp "$PKG_DIR/$APP_NAME.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/"

    # AppRun script
    cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/bash
SELF="$(readlink -f "$0")"
HERE="${SELF%/*}"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"

# First run: offer to install autostart
AUTOSTART_DIR="$HOME/.config/autostart"
AUTOSTART_FILE="$AUTOSTART_DIR/soundblaster-g6x.desktop"
if [ "$1" = "--install-autostart" ]; then
    mkdir -p "$AUTOSTART_DIR"
    cat > "$AUTOSTART_FILE" << DESKTOP
[Desktop Entry]
Name=Sound Blaster G6X Controller
Comment=Linux controller for Creative Sound Blaster G6/G6X USB DAC
Exec=$SELF --minimized
Icon=soundblaster-g6x
Type=Application
Terminal=false
StartupNotify=false
X-GNOME-Autostart-enabled=true
DESKTOP
    echo "Autostart installed: $AUTOSTART_FILE"
    exit 0
fi

if [ "$1" = "--remove-autostart" ]; then
    rm -f "$AUTOSTART_FILE"
    echo "Autostart removed."
    exit 0
fi

exec "${HERE}/usr/bin/soundblaster-g6x" "$@"
APPRUN
    chmod 755 "$APPDIR/AppRun"

    # Download appimagetool if not available
    APPIMAGETOOL="$OUTPUT_DIR/appimagetool-x86_64.AppImage"
    if [ ! -f "$APPIMAGETOOL" ]; then
        info "Downloading appimagetool..."
        wget -q "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" -O "$APPIMAGETOOL" 2>&1
        chmod +x "$APPIMAGETOOL"
    fi

    # Build AppImage
    APPIMAGE_FILE="$OUTPUT_DIR/${APP_NAME}-${VERSION}-x86_64.AppImage"
    ARCH=x86_64 "$APPIMAGETOOL" "$APPDIR" "$APPIMAGE_FILE" 2>&1
    rm -rf "$APPDIR"

    ok "AppImage: $APPIMAGE_FILE ($(du -sh "$APPIMAGE_FILE" | cut -f1))"
}

# ============================================
# 3. BUILD Flatpak
# ============================================
build_flatpak() {
    info "Building Flatpak..."

    # Check flatpak-builder
    if ! command -v flatpak-builder &>/dev/null; then
        err "flatpak-builder not found. Install with: sudo apt install flatpak-builder"
        return 1
    fi

    FLATPAK_DIR="$OUTPUT_DIR/flatpak-build"
    FLATPAK_REPO="$OUTPUT_DIR/flatpak-repo"
    rm -rf "$FLATPAK_DIR" "$FLATPAK_REPO"

    # Flatpak manifest
    cat > "$OUTPUT_DIR/cc.dreamzone.SoundBlasterG6X.yml" << EOF
app-id: cc.dreamzone.SoundBlasterG6X
runtime: org.gnome.Platform
runtime-version: '47'
sdk: org.gnome.Sdk
command: soundblaster-g6x
finish-args:
  - --share=ipc
  - --socket=x11
  - --socket=wayland
  - --socket=pulseaudio
  - --device=all
  - --talk-name=org.freedesktop.Notifications
  - --talk-name=org.kde.StatusNotifierWatcher
  - --talk-name=org.freedesktop.portal.Background

modules:
  - name: soundblaster-g6x
    buildsystem: simple
    build-commands:
      - install -Dm755 soundblaster-g6x /app/bin/soundblaster-g6x
      - install -Dm644 soundblaster-g6x.desktop /app/share/applications/cc.dreamzone.SoundBlasterG6X.desktop
      - install -Dm644 soundblaster-g6x.png /app/share/icons/hicolor/256x256/apps/soundblaster-g6x.png
      - install -Dm755 libxdo.so.3 /app/lib/libxdo.so.3
      - install -Dm755 libayatana-appindicator3.so.1 /app/lib/libayatana-appindicator3.so.1
      - install -Dm755 libayatana-ido3-0.4.so.0 /app/lib/libayatana-ido3-0.4.so.0
      - install -Dm755 libayatana-indicator3.so.7 /app/lib/libayatana-indicator3.so.7
      - install -Dm755 libdbusmenu-glib.so.4 /app/lib/libdbusmenu-glib.so.4
      - install -Dm755 libdbusmenu-gtk3.so.4 /app/lib/libdbusmenu-gtk3.so.4
    sources:
      - type: file
        path: ../target/release/$BINARY_NAME
        dest-filename: soundblaster-g6x
      - type: file
        path: ../packaging/soundblaster-g6x.desktop
      - type: file
        path: ../packaging/soundblaster-g6x.png
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libxdo.so.3
        dest-filename: libxdo.so.3
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libayatana-appindicator3.so.1
        dest-filename: libayatana-appindicator3.so.1
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libayatana-ido3-0.4.so.0
        dest-filename: libayatana-ido3-0.4.so.0
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libayatana-indicator3.so.7
        dest-filename: libayatana-indicator3.so.7
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libdbusmenu-glib.so.4
        dest-filename: libdbusmenu-glib.so.4
      - type: file
        path: /usr/lib/x86_64-linux-gnu/libdbusmenu-gtk3.so.4
        dest-filename: libdbusmenu-gtk3.so.4
EOF

    # Build flatpak
    cd "$OUTPUT_DIR"
    flatpak-builder --force-clean "$FLATPAK_DIR" "cc.dreamzone.SoundBlasterG6X.yml" 2>&1

    # Export to repo and build bundle
    flatpak-builder --repo="$FLATPAK_REPO" --force-clean "$FLATPAK_DIR" "cc.dreamzone.SoundBlasterG6X.yml" 2>&1
    FLATPAK_FILE="$OUTPUT_DIR/${APP_NAME}-${VERSION}.flatpak"
    flatpak build-bundle "$FLATPAK_REPO" "$FLATPAK_FILE" cc.dreamzone.SoundBlasterG6X 2>&1

    rm -rf "$FLATPAK_DIR" "$FLATPAK_REPO"
    ok "Flatpak: $FLATPAK_FILE ($(du -sh "$FLATPAK_FILE" | cut -f1))"
}

# ============================================
# MAIN
# ============================================
echo ""
echo "============================================"
echo "  $APP_LABEL - Package Builder v$VERSION"
echo "============================================"
echo ""

case "${1:-all}" in
    deb)      build_deb ;;
    appimage) build_appimage ;;
    flatpak)  build_flatpak ;;
    all)
        build_deb
        echo ""
        build_appimage
        echo ""
        build_flatpak
        ;;
    *)
        echo "Usage: $0 {deb|appimage|flatpak|all}"
        exit 1
        ;;
esac

echo ""
echo "============================================"
info "All packages in: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"/*.{deb,AppImage,flatpak} 2>/dev/null || true
echo "============================================"
