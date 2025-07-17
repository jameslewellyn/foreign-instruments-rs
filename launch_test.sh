#!/bin/bash

set -e

# Foreign Instruments RS Test Launch Script
# Builds and runs the test with logging and environment setup

echo "🚀 Launching Foreign Instruments RS Test"
echo "========================================"

# Check for Cargo.toml
if [ ! -f "Cargo.toml" ]; then
  echo "[ERROR] Cargo.toml not found. Please run this script from the project root directory."
  exit 1
fi

# Check for cargo
if ! command -v cargo &> /dev/null; then
  echo "[ERROR] cargo not found. Please install Rust: https://rustup.rs/"
  exit 1
fi

echo "[INFO] Building project..."
cargo build --release

echo "[INFO] Build completed."

# Set environment variables (can be overridden)
export RUST_LOG=${RUST_LOG:-"info,foreigninstruments=debug"}
export USB_POLL_INTERVAL=${USB_POLL_INTERVAL:-100}
export MIDI_PORT_NAME=${MIDI_PORT_NAME:-"Foreign Instruments Bridge"}

echo "[INFO] Environment variables:"
echo "  RUST_LOG=$RUST_LOG"
echo "  USB_POLL_INTERVAL=$USB_POLL_INTERVAL"
echo "  MIDI_PORT_NAME=$MIDI_PORT_NAME"

echo "[INFO] Starting Foreign Instruments Bridge..."
echo ""
echo "📋 Test Information:"
echo "  - USB devices will be listed on startup"
echo "  - Hotplug events will be detected for Native Instruments devices"
echo "  - USB messages will be translated to MIDI"
echo "  - MIDI output will be sent to virtual port: $MIDI_PORT_NAME"
echo "  - Simulation mode is enabled as fallback"
echo ""
echo "🔧 To test with real devices:"
echo "  - Connect a Native Instruments device (Komplete Kontrol, Maschine, etc.)"
echo "  - The device should appear in the device list"
echo "  - USB messages will be read and translated to MIDI"
echo ""
echo "🎵 To monitor MIDI output:"
echo "  - Use a DAW or MIDI monitor to connect to '$MIDI_PORT_NAME'"
echo "  - Or use: midi-monitor (macOS) or similar tools"
echo ""
echo "Press Ctrl+C to stop the test."
echo ""

# Run the application
./target/release/foreigninstrumentsd 