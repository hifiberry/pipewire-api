# PipeWire API

A REST API server for controlling PipeWire audio parameters and a command-line tool for parameter manipulation.

## Overview

This project provides:
- **pipewire-api**: REST API server for controlling PipeWire audio processing
- **pw-param**: Command-line tool for reading and writing PipeWire parameters

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
- Man pages for both tools
- Systemd user service

## Usage

### API Server

Start the service:
```bash
systemctl --user enable --now pipewire-api
```

### Command Line Tool

```bash
pw-param --help
```

## Debian Package

Build a Debian package:
```bash
make deb
```

## Documentation

See [api.md](api.md) for API documentation.

## License

See LICENSE file for details.
