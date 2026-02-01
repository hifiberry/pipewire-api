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

API documentation is split into separate files for each module:

- [docs/README.md](docs/README.md) - API documentation index
- [docs/API_CORE.md](docs/API_CORE.md) - Core endpoints (listing, properties, cache)
- [docs/API_VOLUME.md](docs/API_VOLUME.md) - Volume management
- [docs/API_LINKS.md](docs/API_LINKS.md) - Link management
- [docs/API_SPEAKEREQ.md](docs/API_SPEAKEREQ.md) - SpeakerEQ module
- [docs/API_RIAA.md](docs/API_RIAA.md) - RIAA phono preamplifier module
- [LINKS_API.md](LINKS_API.md) - Link rules documentation (experimental)
- [link-rules.conf.md](link-rules.conf.md) - Link rules configuration format

## API Endpoints

The API server provides several categories of endpoints:

### Generic PipeWire Inspection
- `/api/v1/ls` - List all PipeWire objects
- `/api/v1/ls/{nodes,devices,ports,modules,factories,clients,links}` - List specific object types
- `/api/v1/properties` - List all objects with properties
- `/api/v1/properties/:id` - Get properties for a specific object

### SpeakerEQ Control
- `/api/v1/module/speakereq/structure` - Get DSP structure
- `/api/v1/module/speakereq/io` - Get I/O configuration
- `/api/v1/module/speakereq/status` - Get current status
- `/api/v1/module/speakereq/eq` - Manage equalizer settings
- `/api/v1/module/speakereq/gain` - Control gain settings
- `/api/v1/module/speakereq/enable` - Enable/disable processing

### RIAA Phono Preamplifier Control
- `/api/v1/module/riaa/config` - Get all RIAA settings
- `/api/v1/module/riaa/gain` - Get/set preamplifier gain
- `/api/v1/module/riaa/subsonic` - Get/set subsonic (rumble) filter
- `/api/v1/module/riaa/riaa-enable` - Enable/disable RIAA equalization
- `/api/v1/module/riaa/declick` - Enable/disable declicker
- `/api/v1/module/riaa/spike` - Configure spike detection
- `/api/v1/module/riaa/notch` - Configure notch filter
- `/api/v1/module/riaa/set-default` - Reset to defaults

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
