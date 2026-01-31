# AI Coding Guidelines for PipeWire API Project

## Project Overview

This project provides a REST API server and command-line tools for controlling PipeWire audio parameters.

## Project Structure

- `src/`: Rust implementation of the API server and tools
  - `pipewire-api.rs`: REST API server for PipeWire control
  - `pw-param.rs`: Command-line parameter manipulation tool
- `tests/`: Test files
- `debian/`: Debian packaging files

## Technology Stack

- Rust (cargo)
- PipeWire library bindings (libspa)
- Actix-web for REST API
- systemd for service management

## Code Style

- Follow Rust standard style (rustfmt)
- Use idiomatic Rust patterns
- Prefer immutability where possible
- Use proper error handling with Result types

## Build System

- Makefile for top-level build orchestration
- Cargo for Rust compilation
- Debian packaging via dpkg-buildpackage

## Testing

- Ensure code compiles without warnings
- Test API endpoints manually or with automated tests in `tests/`
- Verify systemd service functionality
- Run the server with `RUST_LOG=info cargo run` to see debug output

## Development

- Use `RUST_LOG=info cargo run` to run the API server with logging enabled
- Default port is 2716, can be changed with `-p` flag
- Do not use `--localhost` flag 

## Documentation

- Update API documentation in `api.md` when adding endpoints
- Keep README.md synchronized with feature changes
- Document all public functions and modules

## Deployment

- Use `make deb` to create Debian packages
- Install via `make install-all`
- Service managed through systemd user units
