#!/usr/bin/env python3
"""
Integration tests for the SpeakerEQ API server.
Tests start the server on a random port >33000 and verify all endpoints.

Some tests are marked with @pytest.mark.local_only and will be skipped
when running against a remote server (tests that verify parameters directly
via pw-cli).
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
    Find any speakereq node (speakereqNxM) dynamically.
    Returns tuple (node_id, node_name) or (None, None) if not found.
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
            # Look for any node.name that matches speakereq pattern
            match = re.search(r'node\.name = "(speakereq\d+x\d+)"', line)
            if match and 'media.class = "Audio/Sink"' in lines[i+1] if i+1 < len(lines) else False:
                node_name = match.group(1)
                # Look backwards for the id line
                for j in range(i-1, max(i-10, 0), -1):
                    if 'id' in lines[j]:
                        id_match = re.search(r'id (\d+)', lines[j])
                        if id_match:
                            return int(id_match.group(1)), node_name
        return None, None
    except Exception as e:
        print(f"Error finding speakereq node: {e}")
        return None, None


def get_pw_param(param_name, node_id=None, node_name=None):
    """
    Read a parameter value directly from PipeWire using pw-cli.
    Returns the parameter value as a string, or None if not found.
    """
    if node_id is None or node_name is None:
        node_id, node_name = find_speakereq_node()
        if node_id is None:
            print("Could not find speakereq node")
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
        #   String "speakereqNxM:parameter_name"
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


# Note: api_server fixture is provided by conftest.py (session-scoped)


@pytest.fixture(scope="module")
def speakereq_server(api_server):
    """
    Module-scoped fixture that ensures the speakereq cache is refreshed.
    Uses the shared api_server from conftest.py.
    """
    import sys
    sys.stderr.write(f"\n=== speakereq_server fixture: api_server = {api_server}\n")
    sys.stderr.flush()
    # Refresh the speakereq cache to ensure parameters are loaded
    response = requests.post(f"{api_server}/api/v1/module/speakereq/refresh")
    sys.stderr.write(f"=== speakereq_server fixture: refresh response = {response.status_code}\n")
    sys.stderr.flush()
    if response.status_code != 200:
        sys.stderr.write(f"=== speakereq_server fixture: skipping - response body: {response.text}\n")
        sys.stderr.flush()
        pytest.skip("Could not refresh speakereq cache - module may not be available")
    return api_server


def test_get_structure(speakereq_server):
    """Test GET /api/v1/module/speakereq/speakereq/structure endpoint"""
    node_id, node_name = find_speakereq_node()
    if node_id is None:
        pytest.skip("No speakereq node found")
    
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/structure")
    assert response.status_code == 200
    
    data = response.json()
    assert "name" in data
    assert isinstance(data["inputs"], int)
    assert isinstance(data["outputs"], int)
    assert data["inputs"] > 0
    assert data["outputs"] > 0
    assert isinstance(data["blocks"], list)
    assert len(data["blocks"]) > 0
    assert isinstance(data["enabled"], bool)
    assert isinstance(data["licensed"], bool)


def test_get_io(speakereq_server):
    """Test GET /api/v1/module/speakereq/speakereq/io endpoint"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/io")
    assert response.status_code == 200
    
    data = response.json()
    assert data["inputs"] == 2
    assert data["outputs"] == 2


def test_get_config(speakereq_server):
    """Test GET /api/v1/module/speakereq/config endpoint - dynamic configuration discovery"""
    # Find the speakereq node to get its name
    node_id, node_name = find_speakereq_node()
    if node_id is None:
        pytest.skip("No speakereq node found")
    
    # Parse the expected inputs/outputs from the node name (speakereqNxM)
    match = re.search(r'speakereq(\d+)x(\d+)', node_name)
    assert match is not None, f"Node name {node_name} doesn't match speakereqNxM pattern"
    
    expected_inputs = int(match.group(1))
    expected_outputs = int(match.group(2))
    
    # Get config from API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/config")
    assert response.status_code == 200
    
    data = response.json()
    
    # Verify basic structure
    assert "inputs" in data
    assert "outputs" in data
    assert "eq_slots" in data
    assert "plugin_name" in data
    assert "method" in data
    
    # Verify inputs/outputs match the plugin name
    assert data["inputs"] == expected_inputs, \
        f"Plugin {node_name} should have {expected_inputs} inputs, got {data['inputs']}"
    assert data["outputs"] == expected_outputs, \
        f"Plugin {node_name} should have {expected_outputs} outputs, got {data['outputs']}"
    
    # Verify plugin name matches
    assert data["plugin_name"] == node_name
    
    # Verify method indicates probing
    assert data["method"] == "probed_from_parameters"
    
    # Verify EQ slots structure
    assert isinstance(data["eq_slots"], dict)
    
    # Check that all expected input/output blocks have EQ slots
    for i in range(expected_inputs):
        block_name = f"input_{i}"
        assert block_name in data["eq_slots"], \
            f"Missing EQ slots for {block_name}"
        assert data["eq_slots"][block_name] >= 10, \
            f"{block_name} should have at least 10 EQ slots, got {data['eq_slots'][block_name]}"
    
    for i in range(expected_outputs):
        block_name = f"output_{i}"
        assert block_name in data["eq_slots"], \
            f"Missing EQ slots for {block_name}"
        assert data["eq_slots"][block_name] >= 10, \
            f"{block_name} should have at least 10 EQ slots, got {data['eq_slots'][block_name]}"
    
    print(f"✓ Config test passed for {node_name}: {expected_inputs}x{expected_outputs} with {data['eq_slots']} EQ slots")


def test_get_enable(speakereq_server):
    """Test GET /api/v1/module/speakereq/enable endpoint"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/enable")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


@pytest.mark.local_only
def test_set_and_get_enable(speakereq_server):
    """Test setting and getting the enable parameter"""
    # Get initial state
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/enable")
    initial_enabled = response.json()["enabled"]
    
    # Toggle it
    new_value = not initial_enabled
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/enable",
        json={"enabled": new_value}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/enable")
    assert response.json()["enabled"] == new_value
    
    # Verify it changed in PipeWire directly
    pw_value = get_pw_param("Enable")
    assert pw_value is not None, "Failed to read Enable parameter from PipeWire"
    pw_enabled = pw_value.lower() == "true"
    assert pw_enabled == new_value, f"PipeWire value {pw_enabled} doesn't match API value {new_value}"
    
    # Restore original value
    requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/enable",
        json={"enabled": initial_enabled}
    )


def test_get_master_gain(speakereq_server):
    """Test GET /api/v1/module/speakereq/gain/master endpoint"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/gain/master")
    assert response.status_code == 200
    
    data = response.json()
    assert "gain" in data
    gain = data["gain"]
    assert -60.0 <= gain <= 12.0


@pytest.mark.local_only
def test_set_and_get_master_gain(speakereq_server):
    """Test setting and getting master gain"""
    # Get initial value
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/gain/master")
    initial_gain = response.json()["gain"]
    
    # Set new value
    test_gain = -6.0
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/gain/master",
        json={"gain": test_gain}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/gain/master")
    new_gain = response.json()["gain"]
    assert abs(new_gain - test_gain) < 0.1, f"Expected {test_gain}, got {new_gain}"
    
    # Verify it changed in PipeWire directly
    pw_value = get_pw_param("master_gain_db")
    assert pw_value is not None, "Failed to read master_gain_db parameter from PipeWire"
    pw_gain = float(pw_value)
    assert abs(pw_gain - test_gain) < 0.1, f"PipeWire value {pw_gain} doesn't match API value {test_gain}"
    
    # Restore original value
    requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/gain/master",
        json={"gain": initial_gain}
    )


def test_invalid_master_gain(speakereq_server):
    """Test that invalid gain values are rejected"""
    # Too low
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/gain/master",
        json={"gain": -100.0}
    )
    assert response.status_code == 400
    
    # Too high
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/gain/master",
        json={"gain": 50.0}
    )
    assert response.status_code == 400


def test_get_eq_band(speakereq_server):
    """Test GET /api/v1/module/speakereq/eq/{block}/{band} endpoint"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/output_0/1")
    assert response.status_code == 200
    
    data = response.json()
    assert "type" in data
    assert "frequency" in data
    assert "q" in data
    assert "gain" in data


@pytest.mark.local_only
def test_set_and_get_eq_band(speakereq_server):
    """Test setting and getting EQ band parameters"""
    block = "output_0"
    band = 5
    
    # Get initial state
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    initial_eq = response.json()
    
    # Set new EQ values
    test_eq = {
        "type": "peaking",
        "frequency": 1000.0,
        "q": 2.5,
        "gain": 3.0
    }
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
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
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=initial_eq
    )


def test_invalid_eq_parameters(speakereq_server):
    """Test that invalid EQ parameters are rejected"""
    block = "output_0"
    band = 1
    
    # Invalid frequency (too low)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 10.0, "q": 1.0, "gain": 0.0}
    )
    assert response.status_code == 400
    
    # Invalid Q (too high)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 1000.0, "q": 20.0, "gain": 0.0}
    )
    assert response.status_code == 400
    
    # Invalid gain (too high)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json={"type": "peaking", "frequency": 1000.0, "q": 1.0, "gain": 50.0}
    )
    assert response.status_code == 400
    
    # Invalid EQ type
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json={"type": "invalid_type", "frequency": 1000.0, "q": 1.0, "gain": 0.0}
    )
    assert response.status_code == 400


def test_all_eq_types(speakereq_server):
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
            f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
            json={"type": eq_type, "frequency": 1000.0, "q": 1.0, "gain": 0.0}
        )
        assert response.status_code == 200, f"Failed to set type {eq_type}"
        
        time.sleep(0.05)
        
        # Verify
        response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
        data = response.json()
        assert data["type"] == eq_type, f"Expected {eq_type}, got {data['type']}"


def test_eq_band_enabled_field(speakereq_server):
    """Test that EQ band GET returns enabled field"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/output_0/1")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


@pytest.mark.local_only
def test_set_eq_band_with_enabled(speakereq_server):
    """Test setting EQ band with enabled field"""
    block = "input_0"
    band = 3
    
    # Get initial state
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
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
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify it changed via API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == False
    
    # Verify it changed in PipeWire directly
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "false", f"PipeWire enabled {pw_enabled} should be false"
    
    # Set with enabled=true
    test_eq["enabled"] = True
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled is now true
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True
    
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "true", f"PipeWire enabled {pw_enabled} should be true"
    
    # Restore original values
    requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=initial_eq
    )


@pytest.mark.local_only
def test_set_eq_band_without_enabled(speakereq_server):
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
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled defaults to true
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True, "Enabled should default to true when not specified"
    
    # Verify in PipeWire
    pw_enabled = get_pw_param(f"{block}_eq_{band}_enabled")
    assert pw_enabled is not None, f"Failed to read {block}_eq_{band}_enabled from PipeWire"
    assert pw_enabled.lower() == "true", f"PipeWire enabled {pw_enabled} should default to true"


@pytest.mark.local_only
def test_dedicated_enabled_endpoint(speakereq_server):
    """Test the dedicated enabled endpoint PUT /api/v1/module/speakereq/eq/{block}/{band}/enabled"""
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
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
        json=test_eq
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Get initial state to verify parameters
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    initial_data = response.json()
    assert initial_data["enabled"] == True
    
    # Use dedicated endpoint to disable the band
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}/enabled",
        json={"enabled": False}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled changed but other parameters remain the same
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
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
        f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}/enabled",
        json={"enabled": True}
    )
    assert response.status_code == 200
    
    time.sleep(0.1)
    
    # Verify enabled changed back
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    data = response.json()
    assert data["enabled"] == True
    
    # Verify other parameters still unchanged
    assert data["type"] == "notch"
    assert abs(data["frequency"] - 5000.0) < 1.0
    assert abs(data["q"] - 3.0) < 0.1
    assert abs(data["gain"] - (-12.0)) < 0.1


def test_status_includes_enabled(speakereq_server):
    """Test that GET /api/v1/module/speakereq/status includes enabled for all EQ bands"""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/status")
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


@pytest.mark.local_only
def test_refresh_cache_after_external_change(speakereq_server):
    """Test that refresh endpoint updates cache after external pw-cli changes"""
    block = "output_0"
    band = 3
    node_id, node_name = find_speakereq_node()
    assert node_id is not None, "Could not find speakereq node"
    
    # Get initial value via API
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    assert response.status_code == 200
    initial_data = response.json()
    
    # Change a parameter directly with pw-cli (outside the API)
    # Set type to high_shelf (2) using pw-cli
    subprocess.run([
        "pw-cli", "set-param", str(node_id), "Props",
        f'{{ "params": ["{node_name}:output_0_eq_3_type", 2] }}'
    ], check=True, capture_output=True)
    
    # Give PipeWire time to process
    time.sleep(0.1)
    
    # Without refresh, API still returns cached (old) value
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    assert response.status_code == 200
    cached_data = response.json()
    # Cache should still have old value
    assert cached_data["type"] == initial_data["type"]
    
    # Now refresh the cache
    response = requests.post(f"{speakereq_server}/api/v1/module/speakereq/refresh")
    assert response.status_code == 200
    refresh_result = response.json()
    assert "message" in refresh_result
    
    # After refresh, API should return the new value
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
    assert response.status_code == 200
    refreshed_data = response.json()
    assert refreshed_data["type"] == "high_shelf", f"Expected 'high_shelf' after refresh, got '{refreshed_data['type']}'"
    
    # Cleanup: set it back to off
    subprocess.run([
        "pw-cli", "set-param", str(node_id), "Props",
        f'{{ "params": ["{node_name}:output_0_eq_3_type", 0] }}'
    ], check=True, capture_output=True)


@pytest.mark.local_only
def test_set_default(speakereq_server):
    """Test setting all parameters to default values"""
    node_id, node_name = find_speakereq_node()
    if node_id is None:
        pytest.skip("speakereq node not found")
    
    # First, set some non-default values and verify they're set
    
    # 1. Set master gain to non-zero
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/gain/master",
        json={"gain": -5.0}
    )
    assert response.status_code == 200
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/gain/master")
    assert response.json()["gain"] == -5.0, "Master gain not set to -5.0"
    
    # 2. Set multiple EQ bands to non-default values
    for block in ["input_0", "output_1"]:
        for band in [1, 5, 10]:
            response = requests.put(
                f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}",
                json={
                    "type": "peaking",
                    "frequency": 2000.0,
                    "q": 2.5,
                    "gain": 6.0,
                    "enabled": True
                }
            )
            assert response.status_code == 200
    
    # Verify EQ was set
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/input_0/1")
    assert response.json()["type"] == "peaking", "EQ not set to peaking"
    
    # 3. Set crossbar to non-identity values using pw-cli directly
    subprocess.run([
        "pw-cli", "set-param", str(node_id), "Props",
        f'{{ "params": ["{node_name}:xbar_0_to_0", 0.5, "{node_name}:xbar_0_to_1", 0.7, "{node_name}:xbar_1_to_0", 0.3, "{node_name}:xbar_1_to_1", 0.8] }}'
    ], check=True, capture_output=True)
    
    # Force cache refresh to see crossbar changes
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/refresh")
    
    # Verify crossbar is NOT identity before default
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/status")
    status = response.json()
    assert status["crossbar"]["input_0_to_output_0"] == 0.5, "Crossbar not set to non-default"
    assert status["crossbar"]["input_0_to_output_1"] == 0.7, "Crossbar not set to non-default"
    
    # 4. Set enable to false
    requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/enable",
        json={"enabled": False}
    )
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/enable")
    assert response.json()["enabled"] == False, "Enable not set to false"
    
    # Now call the default endpoint
    response = requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")
    assert response.status_code == 200
    
    data = response.json()
    assert data["status"] == "ok"
    assert "message" in data
    
    # Verify all defaults are set correctly
    
    # Verify master gain is 0dB
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/gain/master")
    assert response.status_code == 200
    assert response.json()["gain"] == 0.0, "Master gain not reset to 0dB"
    
    # Verify all EQ bands are set to off
    for block in ["input_0", "input_1", "output_0", "output_1"]:
        for band in [1, 5, 10, 20]:
            response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/eq/{block}/{band}")
            assert response.status_code == 200
            eq_data = response.json()
            assert eq_data["type"] == "off", f"EQ {block}/{band} not set to off"
            assert eq_data["enabled"] == True, f"EQ {block}/{band} enabled not set to true"
    
    # Verify enable is true
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/enable")
    assert response.status_code == 200
    assert response.json()["enabled"] == True, "Enable not reset to true"
    
    # Verify crossbar is identity matrix
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/status")
    assert response.status_code == 200
    status = response.json()
    assert status["crossbar"]["input_0_to_output_0"] == 1.0, "Crossbar [0,0] not 1.0"
    assert status["crossbar"]["input_0_to_output_1"] == 0.0, "Crossbar [0,1] not 0.0"
    assert status["crossbar"]["input_1_to_output_0"] == 0.0, "Crossbar [1,0] not 0.0"
    assert status["crossbar"]["input_1_to_output_1"] == 1.0, "Crossbar [1,1] not 1.0"


def test_get_crossbar_matrix(speakereq_server):
    """Test getting crossbar matrix in 2D array format."""
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/crossbar")
    assert response.status_code == 200
    
    data = response.json()
    assert "matrix" in data
    matrix = data["matrix"]
    
    # Verify it's a 2x2 matrix
    assert len(matrix) == 2, "Matrix should have 2 rows (inputs)"
    assert len(matrix[0]) == 2, "Matrix row 0 should have 2 columns (outputs)"
    assert len(matrix[1]) == 2, "Matrix row 1 should have 2 columns (outputs)"
    
    # All values should be floats
    for i in range(2):
        for j in range(2):
            assert isinstance(matrix[i][j], (int, float)), f"Matrix[{i}][{j}] should be numeric"


def test_set_crossbar_single_value(speakereq_server):
    """Test setting a single crossbar value."""
    # First reset to identity
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")
    time.sleep(0.1)
    
    # Set crossbar[0][1] to 0.5
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar/0/1",
        json={"value": 0.5}
    )
    assert response.status_code == 200
    
    data = response.json()
    assert data["success"] is True
    assert data["input"] == 0
    assert data["output"] == 1
    assert data["value"] == 0.5
    
    # Verify the change
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/crossbar")
    matrix = response.json()["matrix"]
    assert matrix[0][1] == 0.5, "Crossbar[0][1] should be 0.5"
    assert matrix[0][0] == 1.0, "Crossbar[0][0] should remain 1.0"
    
    # Reset to identity
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")


def test_set_crossbar_single_value_validation(speakereq_server):
    """Test validation for single crossbar value updates."""
    # Test out of range value (> 2.0)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar/0/0",
        json={"value": 2.5}
    )
    assert response.status_code == 400
    assert "out of range" in response.json()["error"].lower() or "0.0 and 2.0" in response.json()["error"]
    
    # Test negative value
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar/0/0",
        json={"value": -0.5}
    )
    assert response.status_code == 400
    
    # Test invalid input index
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar/2/0",
        json={"value": 1.0}
    )
    assert response.status_code == 400
    
    # Test invalid output index
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar/0/5",
        json={"value": 1.0}
    )
    assert response.status_code == 400


def test_set_crossbar_bulk_matrix(speakereq_server):
    """Test setting entire crossbar matrix in one request."""
    # First reset to identity
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")
    time.sleep(0.1)
    
    # Set a custom matrix
    test_matrix = [
        [0.8, 0.2],
        [0.3, 0.7]
    ]
    
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": test_matrix}
    )
    assert response.status_code == 200
    
    data = response.json()
    assert data["success"] is True
    assert data["matrix"] == test_matrix
    
    # Verify the change persisted
    time.sleep(0.1)
    response = requests.get(f"{speakereq_server}/api/v1/module/speakereq/crossbar")
    matrix = response.json()["matrix"]
    
    assert matrix[0][0] == 0.8, "Crossbar[0][0] should be 0.8"
    assert matrix[0][1] == 0.2, "Crossbar[0][1] should be 0.2"
    assert matrix[1][0] == 0.3, "Crossbar[1][0] should be 0.3"
    assert matrix[1][1] == 0.7, "Crossbar[1][1] should be 0.7"
    
    # Reset to identity
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")


def test_set_crossbar_bulk_validation(speakereq_server):
    """Test validation for bulk crossbar matrix updates."""
    # Test wrong matrix dimensions - too many rows
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[1.0, 0.0], [0.0, 1.0], [0.5, 0.5]]}
    )
    assert response.status_code == 400
    assert "2 input rows" in response.json()["error"] or "exactly 2" in response.json()["error"]
    
    # Test wrong matrix dimensions - wrong number of columns
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[1.0, 0.0, 0.5], [0.0, 1.0, 0.5]]}
    )
    assert response.status_code == 400
    assert "2 output columns" in response.json()["error"] or "exactly 2" in response.json()["error"]
    
    # Test out of range value in matrix
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[1.0, 2.5], [0.0, 1.0]]}
    )
    assert response.status_code == 400
    assert "out of range" in response.json()["error"].lower() or "0.0-2.0" in response.json()["error"]
    
    # Test negative value in matrix
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[1.0, 0.0], [-0.5, 1.0]]}
    )
    assert response.status_code == 400


def test_crossbar_edge_cases(speakereq_server):
    """Test crossbar edge cases with valid boundary values."""
    # Test all zeros (valid)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[0.0, 0.0], [0.0, 0.0]]}
    )
    assert response.status_code == 200
    
    # Test all maximum values (2.0)
    response = requests.put(
        f"{speakereq_server}/api/v1/module/speakereq/crossbar",
        json={"matrix": [[2.0, 2.0], [2.0, 2.0]]}
    )
    assert response.status_code == 200
    
    # Verify
    matrix = requests.get(f"{speakereq_server}/api/v1/module/speakereq/crossbar").json()["matrix"]
    assert all(matrix[i][j] == 2.0 for i in range(2) for j in range(2))
    
    # Reset to identity
    requests.post(f"{speakereq_server}/api/v1/module/speakereq/default")


def test_save(speakereq_server):
    """Test that the save endpoint returns a successful response."""
    response = requests.post(f"{speakereq_server}/api/v1/module/speakereq/save")
    assert response.status_code == 200
    
    data = response.json()
    assert data.get("status") == "ok"
    assert "message" in data


if __name__ == "__main__":
    # Allow running tests directly
    pytest.main([__file__, "-v"])

