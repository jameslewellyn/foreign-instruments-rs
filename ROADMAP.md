# foreign-instruments-rs Roadmap

## Vision

Enable seamless integration and control of non-MIDI-class-compliant musical instruments (especially Native Instruments hardware) on modern systems using Rust. Provide a robust, extensible, and cross-platform interface for advanced device management, MIDI translation, and real-time interaction.

## High-Level Goals

1. **Device Discovery & Management**

    - Enumerate all connected USB and HID devices
    - Identify and filter supported instrument hardware (Native Instruments, etc.)
    - Support hotplug (dynamic connect/disconnect)

2. **MIDI Translation Layer**

    - Translate proprietary USB/HID protocols to standard MIDI messages
    - Expose a virtual MIDI port for DAWs and other software

3. **Advanced Device Features**

    - Support for LEDs, displays, smart strips, and other non-MIDI controls
    - Firmware update and diagnostics interface

4. **Cross-Platform Support**

    - Linux, macOS, and Windows compatibility

5. **Extensibility & Community**
    - Modular architecture for adding new devices and backends
    - Clear documentation and contribution guidelines

---

## Milestones & Plan

### Milestone 1: Core Device Support ✅

-   [x] Modernize codebase (Rust 2021, rusb, async/await)
-   [x] Enumerate and list all USB devices
-   [x] Filter and highlight Native Instruments and common MIDI devices
-   [x] Implement hotplug support (device connect/disconnect events)

### Milestone 2: HID Support & Event-Driven Architecture 🎯

-   [ ] **Add HID device support** (critical for Maschine Jam and other Native Instruments devices)
    -   Implement HID device enumeration and filtering
    -   Support for HID event parsing and translation
    -   Handle HID input/output channels separately
-   [ ] **Replace polling loops with event-driven architecture**
    -   Remove all polling loops for device communication
    -   Implement proper event callbacks for device input/output
    -   Use async/await for non-blocking device operations
-   [ ] **Multi-threaded input/output architecture**
    -   Separate input and output channels into dedicated threads
    -   Input thread: Handle HID/USB events from devices
    -   Output thread: Send MIDI messages and device feedback
    -   Communication between threads via message passing
    -   Maximum efficiency for real-time performance

### Milestone 3: Device Abstraction & Management

-   [ ] Define trait-based abstraction for instruments, accessors, and detectors
-   [ ] Implement device state management (active, disconnected, error)
-   [ ] Add logging and error reporting
-   [ ] **Device-specific HID protocol implementations**
    -   Maschine Jam HID protocol support
    -   Other Native Instruments device protocols
    -   Generic HID device fallback

### Milestone 4: MIDI Translation Layer

-   [ ] **HID-to-MIDI translation engine**
    -   Translate HID events to MIDI messages
    -   Support for complex HID event sequences
    -   Configurable mapping for different device types
-   [ ] Expose a virtual MIDI port (using e.g. midir, virtual_midi, or OS APIs)
-   [ ] Provide configuration for mapping and filtering
-   [ ] **Real-time MIDI output with minimal latency**

### Milestone 5: Advanced Features

-   [ ] **Bidirectional HID communication**
    -   Support for device feedback (LEDs, displays, smart strips)
    -   Handle device state synchronization
-   [ ] Implement firmware update and diagnostics interface
-   [ ] Add test suite for device emulation and protocol validation

### Milestone 6: Cross-Platform & Community

-   [ ] Ensure Linux, macOS, and Windows support
-   [ ] Write comprehensive documentation and usage examples
-   [ ] Create contribution guidelines and issue templates

---

## Technical Tasks (Next Steps)

### Immediate Priority: HID Support

-   [ ] **Research and implement HID device enumeration**
    -   Use `hidapi` or similar crate for HID support
    -   Add HID device filtering for Native Instruments devices
-   [ ] **Implement HID event handling**
    -   Parse HID input reports from devices
    -   Handle HID output reports for device feedback
-   [ ] **Maschine Jam specific implementation**
    -   Document and implement Maschine Jam HID protocol
    -   Create device-specific event handlers

### Architecture Improvements

-   [ ] **Remove polling loops**
    -   Replace all `poll_usb_devices()` calls with event-driven approach
    -   Implement proper async device communication
-   [ ] **Multi-threaded architecture**
    -   Create separate input and output threads
    -   Implement thread-safe message passing
    -   Ensure real-time performance for MIDI output
-   [ ] **Event-driven device management**
    -   Use proper event callbacks for device connect/disconnect
    -   Implement async device state management

### Device Support

-   [ ] **Expand HID device database**
    -   Add support for more Native Instruments devices
    -   Implement generic HID device fallback
-   [ ] **Device-specific protocol implementations**
    -   Maschine Jam, Komplete Kontrol, etc.
    -   Configurable protocol mapping

### Testing & Documentation

-   [ ] Write integration and unit tests
-   [ ] Add HID device simulation for testing
-   [ ] Document HID protocol implementations
-   [ ] Prepare for first public release (v0.1.0)

---

## Architecture Notes

### Event-Driven Design

-   **No polling loops**: All device communication should be event-driven
-   **Async/await**: Use Rust's async features for non-blocking operations
-   **Callbacks**: Implement proper callback mechanisms for device events

### Multi-Threaded Performance

-   **Input Thread**: Dedicated thread for receiving HID/USB events
-   **Output Thread**: Dedicated thread for sending MIDI messages
-   **Message Passing**: Use channels for communication between threads
-   **Real-time Priority**: Ensure MIDI output has minimal latency

### HID Protocol Support

-   **Maschine Jam**: Primary target device with specific HID protocol
-   **Generic HID**: Fallback support for other HID devices
-   **Bidirectional**: Support for both input and output HID reports

---

_This roadmap is a living document. Contributions and suggestions are welcome!_
