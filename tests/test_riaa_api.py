import requests
import pytest
import subprocess
from pipewire_utils import get_pipewire_param, verify_param_set


# Note: api_server fixture is provided by conftest.py (session-scoped)


def find_riaa_node(base_url):
    """Find RIAA node in the PipeWire graph."""
    response = requests.get(f"{base_url}/api/v1/ls")
    assert response.status_code == 200
    data = response.json()
    
    objects = data.get("objects", [])
    for obj in objects:
        if obj.get("name") == "riaa" and obj.get("type") == "node":
            return obj["id"]
    
    pytest.skip("RIAA node not found")


def test_find_riaa_node(api_server):
    """Test that we can find the RIAA node."""
    node_id = find_riaa_node(api_server)
    assert node_id is not None
    assert isinstance(node_id, int)


def test_get_config(api_server):
    """Test getting RIAA configuration."""
    find_riaa_node(api_server)  # Ensure node exists
    
    response = requests.get(f"{api_server}/api/module/riaa/config")
    assert response.status_code == 200
    
    config = response.json()
    assert "gain_db" in config
    assert "subsonic_filter" in config
    assert "riaa_enable" in config
    assert "declick_enable" in config
    assert "spike_threshold_db" in config
    assert "spike_width_ms" in config
    assert "notch_filter_enable" in config
    assert "notch_frequency_hz" in config
    assert "notch_q_factor" in config


def test_set_default(api_server):
    """Test setting RIAA to default values and verify they persist."""
    node_id = find_riaa_node(api_server)
    
    # First set some non-default values
    requests.put(f"{api_server}/api/module/riaa/gain", json={"gain_db": 5.0})
    requests.put(f"{api_server}/api/module/riaa/subsonic", json={"filter": 1})
    requests.put(f"{api_server}/api/module/riaa/riaa-enable", json={"enabled": True})
    requests.put(f"{api_server}/api/module/riaa/declick", json={"enabled": True})
    
    # Reset to defaults
    response = requests.put(f"{api_server}/api/module/riaa/set-default")
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    
    # Verify defaults are actually set in PipeWire
    assert verify_param_set(node_id, "riaa:Gain (dB)", 0.0), \
        "Gain not reset to 0.0 dB"
    assert verify_param_set(node_id, "riaa:Subsonic Filter", 0), \
        "Subsonic filter not reset to 0"
    assert verify_param_set(node_id, "riaa:RIAA Enable", False), \
        "RIAA Enable not reset to False"
    assert verify_param_set(node_id, "riaa:Declick Enable", False), \
        "Declick Enable not reset to False"


def test_get_gain(api_server):
    """Test getting RIAA gain."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/gain")
    assert response.status_code == 200
    
    data = response.json()
    assert "gain_db" in data
    assert isinstance(data["gain_db"], (int, float))


def test_set_gain(api_server):
    """Test setting RIAA gain and verify it persists in PipeWire."""
    node_id = find_riaa_node(api_server)
    
    # Set gain to 3.5 dB via API
    response = requests.put(f"{api_server}/api/module/riaa/gain", json={"gain_db": 3.5})
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["gain_db"] == 3.5
    
    # Verify the value actually persists in PipeWire
    assert verify_param_set(node_id, "riaa:Gain (dB)", 3.5), \
        "Gain parameter was not set in PipeWire"


def test_get_subsonic_filter(api_server):
    """Test getting RIAA subsonic filter setting."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/subsonic")
    assert response.status_code == 200
    
    data = response.json()
    assert "filter" in data
    assert isinstance(data["filter"], int)


def test_set_subsonic_filter(api_server):
    """Test setting RIAA subsonic filter and verify it persists."""
    node_id = find_riaa_node(api_server)
    
    response = requests.put(f"{api_server}/api/module/riaa/subsonic", json={"filter": 1})
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["filter"] == 1
    
    # Verify the value persists in PipeWire
    assert verify_param_set(node_id, "riaa:Subsonic Filter", 1), \
        "Subsonic filter not set in PipeWire"


def test_get_riaa_enable(api_server):
    """Test getting RIAA enable status."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/riaa-enable")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


def test_set_riaa_enable(api_server):
    """Test setting RIAA enable and verify it persists."""
    node_id = find_riaa_node(api_server)
    
    response = requests.put(f"{api_server}/api/module/riaa/riaa-enable", json={"enabled": True})
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["enabled"] == True
    
    # Verify the value persists in PipeWire
    assert verify_param_set(node_id, "riaa:RIAA Enable", True), \
        "RIAA Enable not set in PipeWire"


def test_get_declick_enable(api_server):
    """Test getting declick enable status."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/declick")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert isinstance(data["enabled"], bool)


def test_set_declick_enable(api_server):
    """Test setting declick enable and verify it persists."""
    node_id = find_riaa_node(api_server)
    
    response = requests.put(f"{api_server}/api/module/riaa/declick", json={"enabled": True})
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["enabled"] == True
    
    # Verify the value persists in PipeWire
    assert verify_param_set(node_id, "riaa:Declick Enable", True), \
        "Declick Enable not set in PipeWire"


def test_get_spike_config(api_server):
    """Test getting spike detection configuration."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/spike")
    assert response.status_code == 200
    
    data = response.json()
    assert "threshold_db" in data
    assert "width_ms" in data
    assert isinstance(data["threshold_db"], (int, float))
    assert isinstance(data["width_ms"], (int, float))


def test_set_spike_config(api_server):
    """Test setting spike detection configuration."""
    find_riaa_node(api_server)
    
    response = requests.put(
        f"{api_server}/api/module/riaa/spike",
        json={"threshold_db": 25.0, "width_ms": 2.0}
    )
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["threshold_db"] == 25.0
    assert result["width_ms"] == 2.0


def test_get_notch_config(api_server):
    """Test getting notch filter configuration."""
    find_riaa_node(api_server)
    
    response = requests.get(f"{api_server}/api/module/riaa/notch")
    assert response.status_code == 200
    
    data = response.json()
    assert "enabled" in data
    assert "frequency_hz" in data
    assert "q_factor" in data
    assert isinstance(data["enabled"], bool)
    assert isinstance(data["frequency_hz"], (int, float))
    assert isinstance(data["q_factor"], (int, float))


def test_set_notch_config(api_server):
    """Test setting notch filter configuration."""
    find_riaa_node(api_server)
    
    response = requests.put(
        f"{api_server}/api/module/riaa/notch",
        json={"enabled": True, "frequency_hz": 300.0, "q_factor": 30.0}
    )
    assert response.status_code == 200
    
    result = response.json()
    assert result["status"] == "ok"
    assert result["enabled"] == True
    assert result["frequency_hz"] == 300.0
    assert result["q_factor"] == 30.0
