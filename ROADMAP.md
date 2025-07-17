# foreign-instruments-rs Roadmap

## Vision

Enable seamless integration and control of non-MIDI-class-compliant musical instruments (especially Native Instruments hardware) on modern systems using Rust. Provide a robust, extensible, and cross-platform interface for advanced device management, MIDI translation, and real-time interaction.

## High-Level Goals

1. **Device Discovery & Management**

    - Enumerate all connected USB devices
    - Identify and filter supported instrument hardware (Native Instruments, etc.)
    - Support hotplug (dynamic connect/disconnect)

2. **MIDI Translation Layer**

    - Translate proprietary USB protocols to standard MIDI messages
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

### Milestone 1: Core USB Device Support

-   [x] Modernize codebase (Rust 2021, rusb, async/await)
-   [x] Enumerate and list all USB devices
-   [x] Filter and highlight Native Instruments and common MIDI devices
-   [ ] Implement hotplug support (device connect/disconnect events)

### Milestone 2: Device Abstraction & Management

-   [ ] Define trait-based abstraction for instruments, accessors, and detectors
-   [ ] Implement device state management (active, disconnected, error)
-   [ ] Add logging and error reporting

### Milestone 3: MIDI Translation Layer

-   [ ] Implement translation from proprietary USB messages to MIDI
-   [ ] Expose a virtual MIDI port (using e.g. midir, virtual_midi, or OS APIs)
-   [ ] Provide configuration for mapping and filtering

### Milestone 4: Advanced Features

-   [ ] Support for device-specific features (LEDs, displays, smart strips)
-   [ ] Implement firmware update and diagnostics interface
-   [ ] Add test suite for device emulation and protocol validation

### Milestone 5: Cross-Platform & Community

-   [ ] Ensure Linux, macOS, and Windows support
-   [ ] Write comprehensive documentation and usage examples
-   [ ] Create contribution guidelines and issue templates

---

## Technical Tasks (Next Steps)

-   [ ] Refactor and document all modules (types, devices, backends)
-   [ ] Implement async device polling and hotplug
-   [ ] Design and implement the MIDI translation engine
-   [ ] Add support for more devices (expand device database)
-   [ ] Write integration and unit tests
-   [ ] Prepare for first public release (v0.1.0)

---

_This roadmap is a living document. Contributions and suggestions are welcome!_
