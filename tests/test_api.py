#!/usr/bin/env python3
"""
Integration tests for the SpeakerEQ API server.
Tests start the server on a random port >33000 and verify all endpoints.
"""

import subprocess
import requests
import time
import random
import socket
import pytest
import signal
import os
import re


def find_speakereq_node():
    """
    Find the speakereq2x2 node ID dynamically.
    Returns the node ID or None if not found.
    """
    try:
        result = subprocess.run(
            ["pw-cli", "list-objects"],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        lines = result.stdout.split('\n')
        for i, line in enumerate(lines):
            if 'node.name = "speakereq2x2"' in line and 'media.class = "Audio/Sink"' in lines[i+1] if i+1 < len(lines) else False:
                # Look backwards for the id line
                for j in range(i-1, max(i-10, 0), -1):
                    if 'id' in lines[j]:
                        match = re.search(r'id (\d+)', lines[j])
                        if match:
                            return int(match.group(1))
        return None
    except Exception as e:
        print(f"Error finding speakereq node: {e}")
        return None


def get_pw_param(param_name, node_id=None, node_name="speakereq2x2"):
    """
    Read a parameter value directly from PipeWire using pw-cli.
    Returns the parameter value as a string, or None if not found.
    """
    if node_id is None:
        node_id = find_speakereq_node()
        if node_id is None:
            print("Could not find speakereq2x2 node")
            return None
    
    try:
        result = subprocess.run(
            ["pw-cli", "enum-params", str(node_id), "Props"],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        # Parse pw-cli output to find the parameter
        # Format is:
        #   String "speakereq2x2:parameter_name"
        #   Type value
        lines = result.stdout.split('\n')
        
        for i, line in enumerate(lines):
            # Look for the parameter name string
            if f'String "{node_name}:{param_name}"' in line:
                # The next line should contain the value
                if i + 1 < len(lines):
                    value_line = lines[i + 1].strip()
                    # Extract value after the type
                    # e.g., "Float 0.000000" or "Bool false" or "String peaking"
                    parts = value_line.split(None, 1)
                    if len(parts) == 2:
                        return parts[1].strip()
        
        return None
    except Exception as e:
        print(f"Error reading PipeWire parameter: {e}")
        return None


def find_free_port():
    """Find a free port above 33000"""
    for port in range(33000, 34000):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.bind(('127.0.0.1', port))
                return port
            except OSError:
                continue
    raise RuntimeError("No free port found")


@pytest.fixture(scope="module")
def api_server():
    """Start the API server for testing"""
    port = find_free_port()
    base_url = f"http://127.0.0.1:{port}"
    
    # Build the server if not already built
    build_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "pipewire-api"],
        cwd=build_dir,
        check=True,
        capture_output=True
    )
    
    # Start the server
    server_path = os.path.join(build_dir, "target", "release", "pipewire-api")
    process = subprocess.Popen(
        [server_path, "--port", str(port), "--localhost"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=os.setsid
    )
    
    # Wait for server to start
    time.sleep(1.0)
    
    # Check if server is running
    if process.poll() is not None:
        stdout, stderr = process.communicate()
        raise RuntimeError(f"Server failed to start:\nStdout: {stdout.decode()}\nStderr: {stderr.decode()}")
    
    yield base_url
    
    # Cleanup: kill the server
    os.killpg(os.getpgid(process.pid), signal.SIGTERM)
    process.wait(timeout=5)


def test_get_structure(api_server):
    """Test GET /api/v1/speakereq/speakereq/structure endpoint"""
    response = requests.get(f"{api_server}/api/v1/speakereq/structure")
    assert response.status_code == 200
    
    data = response.json()
    assert data["name"] == "speakereq2x2"
    assert data["inputs"] == 2
    assert data["outputs"] == 2
    assert isinstance(data["blocks"], list)
    assert len(data["blocks"]) > 0
    assert isinstance(data["enabled"], bool)
    assert isinstance(data["licensed"], bool)


def test_get_io(api_server):
    """Test GET /api/v1/speakereq/speakereq/io endpoint"""
    response = requests.get(f"{api_server}/api/v1/io")
    assert response.status_code == 200
    
    data = response.json()
    assert data["inputs"] == 2
    assert data["outputs"] == 2


def test_get_enable(api_server):
    """Test GET /api/v1/speakereq/enable endpoint"""
    response = requests.get(f"{api_server}/api/v1/enable")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


def test_set_and_get_enable(api_server):
    """Test setting and getting the enable parameter"""
    # Get initial state
    response = requests.get(f"{api_server}/api/v1/enable")
    initial_enabled = response.json()["enabled"]
    
    # Toggle it
    new_value = not initial_enabled
    response = requests.put(
        f"{api_server}/api/v1/enable",
        json={"enabled": new_value}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{api_server}/api/v1/enable")
    assert response.json()["enabled"] == new_value
    
    # Verify it changed in PipeWire directly
    pw_value = get_pw_param("Enable")
    assert pw_value is not None, "Failed to read Enable parameter from PipeWire"
    pw_enabled = pw_value.lower() == "true"
    assert pw_enabled == new_value, f"PipeWire value {pw_enabled} doesn't match API value {new_value}"
    
    # Restore original value
    requests.put(
        f"{api_server}/api/v1/enable",
        json={"enabled": initial_enabled}
    )


def test_get_master_gain(api_server):
    """Test GET /api/v1/speakereq/gain/master endpoint"""
    response = requests.get(f"{api_server}/api/v1/gain/master")
    assert response.status_code == 200
    
    data = response.json()
    assert "gain" in data
    gain = data["gain"]
    assert -60.0 <= gain <= 12.0


def test_set_and_get_master_gain(api_server):
    """Test setting and getting master gain"""
    # Get initial value
    response = requests.get(f"{api_server}/api/v1/gain/master")
    initial_gain = response.json()["gain"]
    
    # Set new value
    test_gain = -6.0
    response = requests.put(
        f"{api_server}/api/v1/gain/master",
        json={"gain": test_gain}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{api_server}/api/v1/gain/master")
    new_gain = response.json()["gain"]
    assert abs(new_gain - test_gain) < 0.1, f"Expected {test_gain}, got {new_gain}"
    
    # Verify it changed in PipeWire directly
    pw_value = get_pw_param("master_gain_db")
    assert pw_value is not None, "Failed to read master_gain_db parameter from PipeWire"
    pw_gain = float(pw_value)
    assert abs(pw_gain - test_gain) < 0.1, f"PipeWire value {pw_gain} doesn't match API value {test_gain}"
    
    # Restore original value
    requests.put(
        f"{api_server}/api/v1/gain/master",
        json={"gain": initial_gain}
    )


def test_invalid_master_gain(api_server):
    """Test that invalid gain values are rejected"""
    # Too low
    response = requests.put(
        f"{api_server}/api/v1/gain/master",
        json={"gain": -100.0}
    )
    assert response.status_code == 400
    
    # Too high
    response = requests.put(
        f"{api_server}/api/v1/gain/master",
        json={"gain": 50.0}
    )
    assert response.status_code == 400


def test_get_eq_band(api_server):
    """Test GET /api/v1/speakereq/eq/{block}/{band} endpoint"""
    response = requests.get(f"{api_server}/api/v1/eq/output_0/1")
    assert response.status_code == 200
    
    data = response.json()
    assert "type" in data
    assert "frequency" in data
    assert "q" in data
    assert "gain" in data


def test_set_and_get_eq_band(api_server):
    """Test setting and getting EQ band parameters"""
    block = "output_0"
    band = 5
    
    # Get initial state
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    initial_eq = response.json()
    
    # Set new EQ values
    test_eq = {
        "type": "peaking",
        "frequency": 1000.0,
        "q": 2.5,
        "gain": 3.0
    }
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["type"] == "peaking"
    assert abs(data["frequency"] - 1000.0) < 1.0
    assert abs(data["q"] - 2.5) < 0.1
    assert abs(data["gain"] - 3.0) < 0.1
    
    # Verify it changed in PipeWire directly
    pw_type = get_pw_param(f"{block}_eq_{band}_type")
    pw_freq = get_pw_param(f"{block}_eq_{band}_f")
    pw_q = get_pw_param(f"{block}_eq_{band}_q")
    pw_gain = get_pw_param(f"{block}_eq_{band}_gain")
    
    assert pw_type is not None, f"Failed to read {block}_eq_{band}_type from PipeWire"
    assert pw_freq is not None, f"Failed to read {block}_eq_{band}_f from PipeWire"
    assert pw_q is not None, f"Failed to read {block}_eq_{band}_q from PipeWire"
    assert pw_gain is not None, f"Failed to read {block}_eq_{band}_gain from PipeWire"
    
    # Convert type integer to string for comparison
    type_map = {
        "0": "off", "1": "low_shelf", "2": "high_shelf", "3": "peaking",
        "4": "low_pass", "5": "high_pass", "6": "band_pass", "7": "notch", "8": "all_pass"
    }
    pw_type_str = type_map.get(pw_type, pw_type)
    
    assert pw_type_str == "peaking", f"PipeWire type {pw_type_str} (raw: {pw_type}) doesn't match"
    assert abs(float(pw_freq) - 1000.0) < 1.0, f"PipeWire frequency {pw_freq} doesn't match"
    assert abs(float(pw_q) - 2.5) < 0.1, f"PipeWire Q {pw_q} doesn't match"
    assert abs(float(pw_gain) - 3.0) < 0.1, f"PipeWire gain {pw_gain} doesn't match"
    
    # Restore original values
    requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=initial_eq
    )


def test_invalid_eq_parameters(api_server):
    """Test that invalid EQ parameters are rejected"""
    block = "output_0"
    band = 1
    
    # Invalid frequency (too low)
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 10.0, "q": 1.0, "gain": 0.0}
    )
    assert response.status_code == 400
    
    # Invalid Q (too high)
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 1000.0, "q": 20.0, "gain": 0.0}
    )
    assert response.status_code == 400
    
    # Invalid gain (too high)
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 1000.0, "q": 1.0, "gain": 50.0}
    )
    assert response.status_code == 400
    
    # Invalid EQ type
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json={"type": "invalid_type", "frequency": 1000.0, "q": 1.0, "gain": 0.0}
    )
    assert response.status_code == 400


def test_all_eq_types(api_server):
    """Test that all EQ types can be set and retrieved"""
    block = "output_0"
    band = 10
    
    eq_types = [
        "off", "low_shelf", "high_shelf", "peaking",
        "low_pass", "high_pass", "band_pass", "notch", "all_pass"
    ]
    
    for eq_type in eq_types:
        # Set EQ type
        response = requests.put(
            f"{api_server}/api/v1/eq/{block}/{band}",
            json={"type": eq_type, "frequency": 1000.0, "q": 1.0, "gain": 0.0}
        )
        assert response.status_code == 200, f"Failed to set type {eq_type}"
        
        time.sleep(0.05)
        
        # Verify
        response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
        data = response.json()
        assert data["type"] == eq_type, f"Expected {eq_type}, got {data['type']}"


def test_eq_band_enabled_field(api_server):
    """Test that EQ band GET returns enabled field"""
    response = requests.get(f"{api_server}/api/v1/eq/output_0/1")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


def test_set_eq_band_with_enabled(api_server):
    """Test setting EQ band with enabled field"""
    block = "input_0"
    band = 3
    
    # Get initial state
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    initial_eq = response.json()
    
    # Set EQ with enabled=false
    test_eq = {
        "type": "peaking",
        "frequency": 2000.0,
        "q": 1.5,
        "gain": 6.0,
        "enabled": False
    }
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == False
    
    # Verify it changed in PipeWire directly
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "false", f"PipeWire enabled {pw_enabled} should be false"
    
    # Set with enabled=true
    test_eq["enabled"] = True
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled is now true
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True
    
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "true", f"PipeWire enabled {pw_enabled} should be true"
    
    # Restore original values
    requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=initial_eq
    )


def test_set_eq_band_without_enabled(api_server):
    """Test that enabled defaults to true when not provided"""
    block = "input_1"
    band = 7
    
    # Set EQ without enabled field (should default to true)
    test_eq = {
        "type": "low_shelf",
        "frequency": 100.0,
        "q": 0.7,
        "gain": -3.0
    }
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled defaults to true
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True, "Enabled should default to true when not specified"
    
    # Verify in PipeWire
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "true", f"PipeWire enabled {pw_enabled} should default to true"


def test_dedicated_enabled_endpoint(api_server):
    """Test the dedicated enabled endpoint PUT /api/v1/speakereq/eq/{block}/{band}/enabled"""
    block = "output_1"
    band = 15
    
    # First set up an EQ band with specific parameters
    test_eq = {
        "type": "notch",
        "frequency": 5000.0,
        "q": 3.0,
        "gain": -12.0,
        "enabled": True
    }
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Get initial state to verify parameters
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    initial_data = response.json()
    assert initial_data["enabled"] == True
    
    # Use dedicated endpoint to disable the band
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}/enabled",
        json={"enabled": False}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled changed but other parameters remain the same
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == False, "Enabled should be false"
    assert data["type"] == "notch", "Type should remain unchanged"
    assert abs(data["frequency"] - 5000.0) < 1.0, "Frequency should remain unchanged"
    assert abs(data["q"] - 3.0) < 0.1, "Q should remain unchanged"
    assert abs(data["gain"] - (-12.0)) < 0.1, "Gain should remain unchanged"
    
    # Verify in PipeWire
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None
    assert pw_enabled.lower() == "false"
    
    # Re-enable using dedicated endpoint
    response = requests.put(
        f"{api_server}/api/v1/eq/{block}/{band}/enabled",
        json={"enabled": True}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled changed back
    response = requests.get(f"{api_server}/api/v1/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True
    
    # Verify other parameters still unchanged
    assert data["type"] == "notch"
    assert abs(data["frequency"] - 5000.0) < 1.0
    assert abs(data["q"] - 3.0) < 0.1
    assert abs(data["gain"] - (-12.0)) < 0.1


def test_status_includes_enabled(api_server):
    """Test that GET /api/v1/speakereq/status includes enabled for all EQ bands"""
    response = requests.get(f"{api_server}/api/v1/status")
    assert response.status_code == 200
    
    data = response.json()
    
    # Check inputs
    for input_block in data["inputs"]:
        assert "eq_bands" in input_block
        for band in input_block["eq_bands"]:
            assert "enabled" in band, f"Band {band['band']} in {input_block['id']} missing enabled field"
            assert isinstance(band["enabled"], bool)
    
    # Check outputs
    for output_block in data["outputs"]:
        assert "eq_bands" in output_block
        for band in output_block["eq_bands"]:
            assert "enabled" in band, f"Band {band['band']} in {output_block['id']} missing enabled field"
            assert isinstance(band["enabled"], bool)


if __name__ == "__main__":
    # Allow running tests directly
    pytest.main([__file__, "-v"])

