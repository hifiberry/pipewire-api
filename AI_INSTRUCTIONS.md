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
- **Files should not exceed 1000 lines** - if a file grows beyond this, refactor it by splitting into separate modules with distinct functional blocks

## Build System

- Makefile for top-level build orchestration
- Cargo for Rust compilation
- Debian packaging via dpkg-buildpackage

## Testing

- Ensure code compiles without warnings
- Test API endpoints manually or with automated tests in `tests/`
- Verify systemd service functionality
- Run the server with `RUST_LOG=info cargo run` to see debug output

## Others
- never put IP addresses ad/or password into files that are managed by git
- do not send any passwords to the AI agent. If password are required, use password files (e.g. sshpass -f)

### Running Python API Tests

The project includes comprehensive Python-based API tests in the `tests/` directory.

**Prerequisites:**
```bash
# Create a virtual environment and install test dependencies
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements-test.txt
```

**Running tests:**
```bash
# Run all tests
python3 -m pytest tests/ -v

# Run a specific test file
python3 -m pytest tests/test_speakereq.py -v

# Run tests with shorter output
python3 -m pytest tests/ -q
```

The test framework automatically:
- Builds the release binary (`cargo build --release`)
- Creates a temporary HOME directory with mock PipeWire tools
- Starts the API server on a random available port
- Runs all tests against that server
- Cleans up after completion

**Note:** Tests use session-scoped fixtures, so all tests share the same server instance for efficiency.

### Running Tests Against a Remote Server

To test a deployed API server on a remote system:

```bash
# Run tests against a remote server
python3 tests/test_remote.py http://X.X.X.X:2716

# With verbose output
python3 tests/test_remote.py http://x.x.x.x:2716 -v

# Run specific test file
python3 tests/test_remote.py http://x.x.x.x:2716 tests/test_speakereq.py
```

Tests marked with `@pytest.mark.local_only` are automatically skipped when running against a remote server. These are tests that:
- Verify parameters directly via pw-cli
- Access local state files
- Require restarting the server

## Development

- Use `RUST_LOG=info cargo run` to run the API server with logging enabled
- Default port is 2716, can be changed with `-p` flag
- Do not use `--localhost` flag
- **Development and debugging should be done locally as much as possible**
- For remote testing/debugging, use sshpass to run commands on target systems:
  ```bash
  sshpass -f ~/.sshpass.XXX ssh XXX <command>
  ```
- To build and deploy to a remote system:
  ```bash
  cd ~/hifiberry-os/packages; rm pw-api/*.deb; ./build-all ; \
    sshpass -f ~/.sshpass.XXX scp pw-api/*.deb XXX:; \
    sshpass -f ~/.sshpass.XXX ssh XXX sudo dpkg -i *.deb; \
    sshpass -f ~/.sshpass.XXX ssh XXX systemctl restart --user pipewire-api
  ```
- Ask the user if you don't know what remote system should be used
- **For development tests, use `cargo build` - do NOT use `build-all` or Debian packaging**
- Debian packaging is only needed when the user explicitly asks for a packaging test or wants to deploy on a remote system

## Documentation

- Update API documentation in `api.md` when adding endpoints
- Keep README.md synchronized with feature changes
- Document all public functions and modules

## Deployment

- Use `make deb` to create Debian packages
- Install via `make install-all`
- Service managed through systemd user units
