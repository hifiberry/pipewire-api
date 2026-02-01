"""
Shared pytest configuration and fixtures for all test modules.

This file provides a single API server instance that is shared across ALL tests.
The server is started once at the beginning of the test session and stopped
at the end, ensuring consistent state and faster test execution.

A temporary HOME directory is used to avoid interfering with user's real config.
"""

import subprocess
import signal
import socket
import time
import os
import atexit
import requests
import pytest
import tempfile
import shutil


# Global server state
_server_process = None
_server_base_url = None
_temp_home = None


def find_free_port():
    """Find a free port by letting the OS assign one"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('127.0.0.1', 0))
        s.listen(1)
        port = s.getsockname()[1]
    return port


def _start_server():
    """Start the API server and return the base URL"""
    global _server_process, _server_base_url, _temp_home
    
    if _server_process is not None:
        return _server_base_url
    
    # Create temporary HOME directory only if we don't have one
    if _temp_home is None or not os.path.exists(_temp_home):
        _temp_home = tempfile.mkdtemp(prefix="pipewire_api_test_")
    
    config_dir = os.path.join(_temp_home, ".config", "pipewire-api")
    state_dir = os.path.join(_temp_home, ".state", "pipewire-api")
    os.makedirs(config_dir, exist_ok=True)
    os.makedirs(state_dir, exist_ok=True)
    
    port = find_free_port()
    _server_base_url = f"http://127.0.0.1:{port}"
    
    # Build the server if not already built
    build_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "pipewire-api"],
        cwd=build_dir,
        check=True,
        capture_output=True
    )
    
    # Start the server with isolated HOME
    server_path = os.path.join(build_dir, "target", "release", "pipewire-api")
    env = os.environ.copy()
    env["HOME"] = _temp_home
    env["RUST_LOG"] = "debug"  # Enable debug logging to trace caching issues
    
    # Log file for debugging
    log_file = os.path.join(_temp_home, "server.log")
    log_handle = open(log_file, 'w')
    
    _server_process = subprocess.Popen(
        [server_path, "--port", str(port), "--localhost"],
        stdout=log_handle,
        stderr=subprocess.STDOUT,
        preexec_fn=os.setsid,
        env=env
    )
    
    # Wait for server to be ready
    max_retries = 50
    for _ in range(max_retries):
        if _server_process.poll() is not None:
            log_handle.close()
            with open(log_file, 'r') as f:
                log_content = f.read()
            raise RuntimeError(f"Server failed to start. Log:\n{log_content}")
        try:
            requests.get(f"{_server_base_url}/api/v1/ls", timeout=0.5)
            break
        except requests.exceptions.ConnectionError:
            time.sleep(0.1)
    else:
        _server_process.terminate()
        raise RuntimeError("Server did not become ready in time")
    
    return _server_base_url


def _stop_server():
    """Stop the API server (does NOT cleanup temp directory)"""
    global _server_process, _server_base_url
    
    if _server_process is not None:
        try:
            os.killpg(os.getpgid(_server_process.pid), signal.SIGTERM)
            _server_process.wait(timeout=5)
        except Exception:
            try:
                _server_process.kill()
            except Exception:
                pass
        _server_process = None
        _server_base_url = None


def _cleanup_temp_home():
    """Cleanup temporary HOME directory"""
    global _temp_home
    if _temp_home and os.path.exists(_temp_home):
        shutil.rmtree(_temp_home, ignore_errors=True)
        _temp_home = None


# Register cleanup at exit (safety net)
atexit.register(_stop_server)
# Disable temp cleanup for debugging
# atexit.register(_cleanup_temp_home)


def pytest_sessionstart(session):
    """Called before test collection - kill any stray servers"""
    try:
        subprocess.run(
            ["pkill", "-9", "-f", "pipewire-api"],
            capture_output=True,
            timeout=5
        )
        time.sleep(0.5)
    except Exception:
        pass


def pytest_sessionfinish(session, exitstatus):
    """Called after all tests complete - stop the shared server and cleanup"""
    _stop_server()
    # Skip cleanup to preserve logs for debugging
    # _cleanup_temp_home()
    # Final cleanup of any stray processes
    try:
        subprocess.run(
            ["pkill", "-9", "-f", "pipewire-api"],
            capture_output=True,
            timeout=5
        )
    except Exception:
        pass


@pytest.fixture(scope="session")
def api_server():
    """
    Session-scoped fixture that provides the API server base URL.
    The server is started once for the entire test session.
    """
    return _start_server()


@pytest.fixture(scope="session")
def test_env():
    """
    Alias for api_server for backward compatibility with tests using test_env.
    Returns an object with base_url attribute.
    """
    class TestEnv:
        def __init__(self, base_url, temp_home):
            self.base_url = base_url
            self.temp_home = temp_home
        
        def read_state_file(self):
            """Read the current state file"""
            state_path = os.path.join(self.temp_home, ".state", "pipewire-api", "volume.state")
            if os.path.exists(state_path):
                import json
                with open(state_path, 'r') as f:
                    return json.load(f)
            return None
        
        def create_state_file(self, state):
            """Create a state file with the given content"""
            import json
            state_path = os.path.join(self.temp_home, ".state", "pipewire-api", "volume.state")
            os.makedirs(os.path.dirname(state_path), exist_ok=True)
            with open(state_path, 'w') as f:
                json.dump(state, f, indent=2)
        
        def create_volume_config(self, config):
            """Create a volume config file with the given content"""
            import json
            config_path = os.path.join(self.temp_home, ".config", "pipewire-api", "volume.conf")
            os.makedirs(os.path.dirname(config_path), exist_ok=True)
            with open(config_path, 'w') as f:
                json.dump(config, f, indent=2)
        
        def stop_server(self):
            """Stop the API server"""
            _stop_server()
        
        def start_server(self):
            """Start the API server"""
            global _server_base_url, _temp_home
            _start_server()
            self.base_url = _server_base_url
            self.temp_home = _temp_home
        
        def read_server_log(self):
            """Read the server log file"""
            log_path = os.path.join(self.temp_home, "server.log")
            if os.path.exists(log_path):
                with open(log_path, 'r') as f:
                    return f.read()
            return None
    
    _start_server()
    return TestEnv(_server_base_url, _temp_home)


