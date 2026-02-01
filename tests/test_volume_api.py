#!/usr/bin/env python3
"""
Integration tests for the Volume API.
Tests start the server on a random port and verify volume endpoints.
Uses a temporary HOME directory to avoid overwriting user config/state files.
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
import json
import tempfile
import shutil


def find_volume_controls():
    """
    Find available volume controls (devices and sinks) dynamically.
    Returns a list of dicts with id, name, object_type.
    """
    controls = []
    try:
        # Find devices with volume control
        result = subprocess.run(
            ["pw-cli", "list-objects"],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        lines = result.stdout.split('\n')
        current_id = None
        current_type = None
        current_name = None
        
        for line in lines:
            # Look for object id
            id_match = re.search(r'id (\d+), type PipeWire:Interface:(\w+)', line)
            if id_match:
                current_id = int(id_match.group(1))
                current_type = id_match.group(2)
                current_name = None
                continue
            
            # Look for device.name or node.name
            if current_id is not None:
                if 'device.name = "' in line:
                    match = re.search(r'device\.name = "([^"]+)"', line)
                    if match and current_type == "Device":
                        current_name = match.group(1)
                        controls.append({
                            "id": current_id,
                            "name": current_name,
                            "object_type": "device"
                        })
                elif 'node.name = "' in line and 'media.class = "Audio/Sink"' in lines[lines.index(line)-1:lines.index(line)+3]:
                    match = re.search(r'node\.name = "([^"]+)"', line)
                    if match and current_type == "Node":
                        current_name = match.group(1)
                        controls.append({
                            "id": current_id,
                            "name": current_name,
                            "object_type": "sink"
                        })
        
        return controls
    except Exception as e:
        print(f"Error finding volume controls: {e}")
        return []


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


class VolumeTestEnvironment:
    """Test environment with isolated HOME directory"""
    
    def __init__(self):
        self.temp_home = None
        self.original_home = os.environ.get('HOME')
        self.server_process = None
        self.base_url = None
        self.port = None
        self.build_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.initial_volumes = {}
    
    def setup(self):
        """Set up the test environment"""
        # Create temporary HOME directory
        self.temp_home = tempfile.mkdtemp(prefix="pipewire_api_test_")
        
        # Create config directory structure
        config_dir = os.path.join(self.temp_home, ".config", "pipewire-api")
        state_dir = os.path.join(self.temp_home, ".state", "pipewire-api")
        os.makedirs(config_dir, exist_ok=True)
        os.makedirs(state_dir, exist_ok=True)
        
        # Find a free port
        self.port = find_free_port()
        self.base_url = f"http://127.0.0.1:{self.port}"
        
        return self
    
    def create_volume_config(self, rules):
        """Create a volume.conf file with the given rules"""
        config_dir = os.path.join(self.temp_home, ".config", "pipewire-api")
        config_path = os.path.join(config_dir, "volume.conf")
        with open(config_path, 'w') as f:
            json.dump(rules, f, indent=2)
        return config_path
    
    def create_state_file(self, states):
        """Create a volume.state file with the given states"""
        state_dir = os.path.join(self.temp_home, ".state", "pipewire-api")
        state_path = os.path.join(state_dir, "volume.state")
        with open(state_path, 'w') as f:
            json.dump(states, f, indent=2)
        return state_path
    
    def read_state_file(self):
        """Read the current state file"""
        state_path = os.path.join(self.temp_home, ".state", "pipewire-api", "volume.state")
        if os.path.exists(state_path):
            with open(state_path, 'r') as f:
                return json.load(f)
        return None
    
    def start_server(self):
        """Start the API server"""
        server_path = os.path.join(self.build_dir, "target", "release", "pipewire-api")
        
        if not os.path.exists(server_path):
            # Build if not exists
            subprocess.run(
                ["cargo", "build", "--release", "--bin", "pipewire-api"],
                cwd=self.build_dir,
                check=True,
                capture_output=True
            )
        
        # Set up environment with temporary HOME
        env = os.environ.copy()
        env["HOME"] = self.temp_home
        env["RUST_LOG"] = "info"
        
        self.server_process = subprocess.Popen(
            [server_path, "-p", str(self.port)],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            preexec_fn=os.setsid
        )
        
        # Wait for server to start
        for _ in range(30):  # Wait up to 3 seconds
            time.sleep(0.1)
            try:
                response = requests.get(f"{self.base_url}/api/v1/volume", timeout=0.5)
                if response.status_code == 200:
                    return True
            except:
                pass
        
        # Check if server failed
        if self.server_process.poll() is not None:
            stdout, stderr = self.server_process.communicate()
            raise RuntimeError(f"Server failed to start:\nStdout: {stdout.decode()}\nStderr: {stderr.decode()}")
        
        return True
    
    def stop_server(self):
        """Stop the API server"""
        if self.server_process:
            try:
                os.killpg(os.getpgid(self.server_process.pid), signal.SIGTERM)
                self.server_process.wait(timeout=5)
            except:
                pass
            self.server_process = None
    
    def save_initial_volumes(self):
        """Save initial volumes to restore after tests"""
        try:
            response = requests.get(f"{self.base_url}/api/v1/volume", timeout=5)
            if response.status_code == 200:
                for vol in response.json():
                    self.initial_volumes[vol["id"]] = vol.get("volume")
        except Exception as e:
            print(f"Warning: Could not save initial volumes: {e}")
    
    def restore_initial_volumes(self):
        """Restore volumes to their initial values"""
        for vol_id, volume in self.initial_volumes.items():
            if volume is not None:
                try:
                    requests.put(
                        f"{self.base_url}/api/v1/volume/{vol_id}",
                        json={"volume": volume},
                        timeout=5
                    )
                except:
                    pass
    
    def cleanup(self):
        """Clean up the test environment"""
        self.restore_initial_volumes()
        self.stop_server()
        
        # Remove temporary HOME directory
        if self.temp_home and os.path.exists(self.temp_home):
            shutil.rmtree(self.temp_home, ignore_errors=True)


@pytest.fixture(scope="module")
def test_env():
    """Set up and tear down the test environment"""
    env = VolumeTestEnvironment()
    env.setup()
    env.start_server()
    env.save_initial_volumes()
    
    yield env
    
    env.cleanup()


@pytest.fixture(scope="module")
def volume_controls(test_env):
    """Get available volume controls"""
    response = requests.get(f"{test_env.base_url}/api/v1/volume")
    assert response.status_code == 200
    controls = response.json()
    
    if not controls:
        pytest.skip("No volume controls available for testing")
    
    return controls


class TestVolumeList:
    """Tests for GET /api/v1/volume endpoint"""
    
    def test_list_volumes_returns_200(self, test_env):
        """Test that listing volumes returns 200"""
        response = requests.get(f"{test_env.base_url}/api/v1/volume")
        assert response.status_code == 200
    
    def test_list_volumes_returns_array(self, test_env):
        """Test that listing volumes returns an array"""
        response = requests.get(f"{test_env.base_url}/api/v1/volume")
        data = response.json()
        assert isinstance(data, list)
    
    def test_volume_objects_have_required_fields(self, test_env, volume_controls):
        """Test that volume objects have required fields"""
        for vol in volume_controls:
            assert "id" in vol, "Volume object missing 'id' field"
            assert "name" in vol, "Volume object missing 'name' field"
            assert "object_type" in vol, "Volume object missing 'object_type' field"
            assert vol["object_type"] in ["device", "sink"], f"Invalid object_type: {vol['object_type']}"
    
    def test_volume_objects_have_volume_field(self, test_env, volume_controls):
        """Test that all returned objects have a volume field"""
        for vol in volume_controls:
            assert "volume" in vol, f"Volume object {vol['id']} missing 'volume' field"
            assert vol["volume"] is not None, f"Volume object {vol['id']} has null volume"
    
    def test_no_properties_field(self, test_env, volume_controls):
        """Test that properties field is not included in response"""
        for vol in volume_controls:
            assert "properties" not in vol, f"Volume object {vol['id']} should not have 'properties' field"


class TestVolumeGetById:
    """Tests for GET /api/v1/volume/:id endpoint"""
    
    def test_get_volume_by_id_returns_200(self, test_env, volume_controls):
        """Test getting volume by ID returns 200"""
        vol = volume_controls[0]
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        assert response.status_code == 200
    
    def test_get_volume_by_id_returns_correct_object(self, test_env, volume_controls):
        """Test getting volume by ID returns the correct object"""
        vol = volume_controls[0]
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        data = response.json()
        
        assert data["id"] == vol["id"]
        assert data["name"] == vol["name"]
        assert data["object_type"] == vol["object_type"]
    
    def test_get_volume_by_id_includes_volume(self, test_env, volume_controls):
        """Test that get by ID includes volume field"""
        vol = volume_controls[0]
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        data = response.json()
        
        assert "volume" in data, "Response missing 'volume' field"
    
    def test_get_volume_by_invalid_id_returns_404(self, test_env):
        """Test getting volume by invalid ID returns 404"""
        response = requests.get(f"{test_env.base_url}/api/v1/volume/99999")
        assert response.status_code == 404


class TestVolumeSet:
    """Tests for PUT /api/v1/volume/:id endpoint"""
    
    def test_set_volume_returns_200(self, test_env, volume_controls):
        """Test setting volume returns 200"""
        vol = volume_controls[0]
        response = requests.put(
            f"{test_env.base_url}/api/v1/volume/{vol['id']}",
            json={"volume": 0.5}
        )
        assert response.status_code == 200
    
    def test_set_and_get_volume(self, test_env, volume_controls):
        """Test that setting volume actually changes it"""
        # Find a sink (node) for testing - they tend to be more reliable
        sink_vol = next((v for v in volume_controls if v["object_type"] == "sink"), None)
        if sink_vol is None:
            pytest.skip("No sink available for volume set test")
        
        vol = sink_vol
        
        # Get initial volume
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        initial_volume = response.json().get("volume")
        
        # Set new volume (different from initial)
        new_volume = 0.6 if initial_volume is None or abs(initial_volume - 0.6) > 0.1 else 0.7
        response = requests.put(
            f"{test_env.base_url}/api/v1/volume/{vol['id']}",
            json={"volume": new_volume}
        )
        assert response.status_code == 200
        
        time.sleep(0.3)  # Wait for volume to be applied
        
        # Verify volume changed
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        current_volume = response.json().get("volume")
        
        # Restore original volume first, then check
        if initial_volume is not None:
            requests.put(
                f"{test_env.base_url}/api/v1/volume/{vol['id']}",
                json={"volume": initial_volume}
            )
        
        assert current_volume is not None, "Volume is None after setting"
        # Allow some tolerance for volume changes
        assert abs(current_volume - new_volume) < 0.05, f"Expected ~{new_volume}, got {current_volume}"
    
    def test_set_volume_by_invalid_id_returns_404(self, test_env):
        """Test setting volume by invalid ID returns 404"""
        response = requests.put(
            f"{test_env.base_url}/api/v1/volume/99999",
            json={"volume": 0.5}
        )
        assert response.status_code == 404


class TestVolumeSave:
    """Tests for POST /api/v1/volume/save endpoints"""
    
    def test_save_all_volumes_returns_200(self, test_env):
        """Test saving all volumes returns 200"""
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save")
        assert response.status_code == 200
    
    def test_save_all_volumes_creates_state_file(self, test_env, volume_controls):
        """Test that saving all volumes creates a state file"""
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save")
        assert response.status_code == 200
        
        state = test_env.read_state_file()
        assert state is not None, "State file was not created"
        assert isinstance(state, list), "State file should contain a list"
    
    def test_save_all_volumes_uses_names_as_keys(self, test_env, volume_controls):
        """Test that state file uses names as keys, not IDs"""
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save")
        assert response.status_code == 200
        
        state = test_env.read_state_file()
        for entry in state:
            assert "name" in entry, "State entry missing 'name' field"
            assert "volume" in entry, "State entry missing 'volume' field"
            assert "id" not in entry, "State entry should not have 'id' field"
    
    def test_save_specific_volume_returns_200(self, test_env, volume_controls):
        """Test saving a specific volume returns 200"""
        vol = volume_controls[0]
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save/{vol['id']}")
        assert response.status_code == 200
    
    def test_save_specific_volume_includes_name_in_response(self, test_env, volume_controls):
        """Test that saving specific volume includes name in response"""
        vol = volume_controls[0]
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save/{vol['id']}")
        data = response.json()
        
        assert data.get("success") == True
        assert "name" in data, "Response should include 'name'"
        assert data["name"] == vol["name"]
    
    def test_save_specific_volume_updates_state_file(self, test_env, volume_controls):
        """Test that saving specific volume updates the state file"""
        vol = volume_controls[0]
        
        # First clear the state file
        state_path = os.path.join(test_env.temp_home, ".state", "pipewire-api", "volume.state")
        if os.path.exists(state_path):
            os.remove(state_path)
        
        # Save specific volume
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save/{vol['id']}")
        assert response.status_code == 200
        
        # Check state file
        state = test_env.read_state_file()
        assert state is not None
        
        # Find the saved entry by name
        saved_entry = next((e for e in state if e["name"] == vol["name"]), None)
        assert saved_entry is not None, f"Volume {vol['name']} not found in state file"


class TestVolumeStateFilePersistence:
    """Tests for state file persistence across server restarts"""
    
    def test_state_file_is_loaded_on_startup(self, test_env, volume_controls):
        """Test that state file values are loaded when server starts"""
        vol = volume_controls[0]
        test_volume = 0.42
        
        # Stop server
        test_env.stop_server()
        
        # Create config with use_state_file enabled
        # Use regex pattern that matches the device name
        device_name_pattern = vol["name"].replace(".", r"\\.").replace("-", r"\\-")
        config = [{
            "name": "Test Volume Rule",
            "object": {
                "device.name" if vol["object_type"] == "device" else "node.name": device_name_pattern
            },
            "volume": 1.0,
            "use_state_file": True
        }]
        test_env.create_volume_config(config)
        
        # Create state file with test volume
        state = [{"name": vol["name"], "volume": test_volume}]
        test_env.create_state_file(state)
        
        # Start server
        test_env.start_server()
        time.sleep(1)  # Wait for volume rules to be applied
        
        # Check volume
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        if response.status_code == 200:
            current_volume = response.json().get("volume")
            if current_volume is not None:
                # Volume should be close to the state file value
                assert abs(current_volume - test_volume) < 0.01, \
                    f"Expected volume {test_volume} from state file, got {current_volume}"


class TestVolumeRoundTrip:
    """End-to-end tests for volume operations"""
    
    def test_volume_round_trip(self, test_env, volume_controls):
        """Test complete volume workflow: get, set, save, restore"""
        vol = volume_controls[0]
        
        # 1. Get initial volume
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        initial_volume = response.json().get("volume")
        
        # 2. Set a different volume
        test_volume = 0.33
        response = requests.put(
            f"{test_env.base_url}/api/v1/volume/{vol['id']}",
            json={"volume": test_volume}
        )
        assert response.status_code == 200
        
        time.sleep(0.2)
        
        # 3. Verify it changed
        response = requests.get(f"{test_env.base_url}/api/v1/volume/{vol['id']}")
        current_volume = response.json().get("volume")
        assert abs(current_volume - test_volume) < 0.01
        
        # 4. Save it
        response = requests.post(f"{test_env.base_url}/api/v1/volume/save/{vol['id']}")
        assert response.status_code == 200
        
        # 5. Verify state file
        state = test_env.read_state_file()
        saved_entry = next((e for e in state if e["name"] == vol["name"]), None)
        assert saved_entry is not None
        assert abs(saved_entry["volume"] - test_volume) < 0.01
        
        # 6. Restore original volume
        if initial_volume is not None:
            requests.put(
                f"{test_env.base_url}/api/v1/volume/{vol['id']}",
                json={"volume": initial_volume}
            )


if __name__ == "__main__":
    # Allow running tests directly
    pytest.main([__file__, "-v"])
