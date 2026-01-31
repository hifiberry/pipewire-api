# PipeWire API

A REST API server for controlling PipeWire audio parameters and a command-line tool for parameter manipulation.

## Overview

This project provides:
- **pipewire-api**: REST API server for controlling PipeWire audio processing
- **pw-param**: Command-line tool for reading and writing PipeWire parameters
- **link-nodes**: Command-line tool for managing PipeWire links

## Features

- RESTful API for PipeWire control
- Command-line parameter management
- Systemd integration for automatic service management

## Building

### Prerequisites

- Rust toolchain (cargo)
- PipeWire development libraries
- libclang (for bindgen)

### Build Commands

```bash
make              # Build API server and tools
make install      # Install binaries and man pages
make install-all  # Install everything including API server
```

## Installation

```bash
sudo make install-all
```

This installs:
- `/usr/bin/pipewire-api` - REST API server
- `/usr/bin/pw-param` - Parameter manipulation tool
- `/usr/bin/link-nodes` - Link management tool
- `/etc/pipewire-api/link-rules.conf` - Default link rules configuration
- Man pages for both tools
- Systemd user service

## Configuration

### Link Rules

The API server can automatically manage PipeWire links based on rules defined in configuration files.

**Configuration file locations** (in order of priority):
1. `~/.config/pipewire-api/link-rules.conf` - User-specific configuration
2. `/etc/pipewire-api/link-rules.conf` - System-wide configuration

If no configuration files are found, the server will use hardcoded default rules.

**Example configuration** (`link-rules.conf`):
```json
[
  {
    "name": "SpeakerEQ to HiFiBerry",
    "source": {
      "node.name": "^speakereq.x.\\.output$"
    },
    "destination": {
      "object.path": "alsa:.*:sndrpihifiberry:.*:playback"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 10
  }
]
```

See `link-rules.conf.md` for detailed documentation on the configuration format.

## Usage

### API Server

Start the service:
```bash
systemctl --user enable --now pipewire-api
```

### Command Line Tool

```bash
pw-param --help
link-nodes --help
```

#### link-nodes - Link Management

Apply default link rules (connects SpeakerEQ output to HiFiBerry playback):
```bash
link-nodes apply-defaults
```

Apply default rules with verbose output:
```bash
link-nodes apply-defaults --verbose
```

## Debian Package

Build a Debian package:
```bash
make deb
```

## Documentation

- [api.md](api.md) - General API documentation
- [LINKS_API.md](LINKS_API.md) - Link management API documentation

## API Endpoints

The API server provides several categories of endpoints:

### Generic PipeWire Inspection
- `/api/v1/ls` - List all PipeWire objects
- `/api/v1/ls/{nodes,devices,ports,modules,factories,clients,links}` - List specific object types
- `/api/v1/properties` - List all objects with properties
- `/api/v1/properties/:id` - Get properties for a specific object

### SpeakerEQ Control
- `/api/v1/speakereq/structure` - Get DSP structure
- `/api/v1/speakereq/io` - Get I/O configuration
- `/api/v1/speakereq/status` - Get current status
- `/api/v1/speakereq/eq` - Manage equalizer settings
- `/api/v1/speakereq/gain` - Control gain settings
- `/api/v1/speakereq/enable` - Enable/disable processing

### Link Management (Experimental)
- `/api/v1/links` - List all active links
- `/api/v1/links/apply` - Apply a single link rule
- `/api/v1/links/batch` - Apply multiple link rules
- `/api/v1/links/default` - Get default link rules
- `/api/v1/links/apply-defaults` - Apply default link rules (SpeakerEQ → HiFiBerry)

See [LINKS_API.md](LINKS_API.md) for detailed link management documentation.

**Note:** Link creation is not yet fully implemented. The API can list existing links and validate rules, but cannot create new links yet.

## License

See LICENSE file for details.
