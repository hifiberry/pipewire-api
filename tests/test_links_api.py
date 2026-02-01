"""
Tests for the Links API endpoints.

These tests verify link creation, listing, and removal using the API endpoints.
All verification is done through the API itself, making these tests suitable
for both local and remote testing.
"""

import time
import pytest
import requests


# Note: test_env fixture is provided by conftest.py (session-scoped)


class TestListLinks:
    """Tests for GET /api/v1/links"""
    
    def test_list_links_returns_200(self, test_env):
        """Test that /api/v1/links returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/links")
        assert response.status_code == 200
    
    def test_list_links_returns_json(self, test_env):
        """Test that /api/v1/links returns JSON"""
        response = requests.get(f"{test_env.base_url}/api/v1/links")
        assert "application/json" in response.headers.get("Content-Type", "")
    
    def test_list_links_has_links_array(self, test_env):
        """Test that response has links array"""
        response = requests.get(f"{test_env.base_url}/api/v1/links")
        data = response.json()
        assert "links" in data
        assert isinstance(data["links"], list)
    
    def test_list_links_structure(self, test_env):
        """Test that links have correct structure"""
        response = requests.get(f"{test_env.base_url}/api/v1/links")
        data = response.json()
        
        if data["links"]:  # Only test if there are links
            link = data["links"][0]
            assert "id" in link
            assert "output_port_id" in link
            assert "output_port_name" in link
            assert "input_port_id" in link
            assert "input_port_name" in link


class TestListPorts:
    """Tests for ports listing endpoints"""
    
    def test_list_output_ports_returns_200(self, test_env):
        """Test that /api/v1/links/ports/output returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/links/ports/output")
        assert response.status_code == 200
    
    def test_list_input_ports_returns_200(self, test_env):
        """Test that /api/v1/links/ports/input returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/links/ports/input")
        assert response.status_code == 200
    
    def test_output_ports_have_structure(self, test_env):
        """Test that output ports have correct structure"""
        response = requests.get(f"{test_env.base_url}/api/v1/links/ports/output")
        data = response.json()
        
        assert "ports" in data
        if data["ports"]:
            port = data["ports"][0]
            assert "id" in port
            assert "name" in port
            assert "node_name" in port
            assert "port_name" in port
    
    def test_input_ports_have_structure(self, test_env):
        """Test that input ports have correct structure"""
        response = requests.get(f"{test_env.base_url}/api/v1/links/ports/input")
        data = response.json()
        
        assert "ports" in data
        if data["ports"]:
            port = data["ports"][0]
            assert "id" in port
            assert "name" in port
            assert "node_name" in port
            assert "port_name" in port


class TestCreateAndRemoveLink:
    """Tests for creating and removing links"""
    
    def find_linkable_ports(self, test_env):
        """Find an output and input port that can be linked for testing.
        
        Filters out:
        - Ports from the same node (can't self-link)
        - Already linked pairs
        - Monitor ports (output only)
        - MIDI ports (incompatible with audio)
        
        Returns a pair of compatible audio ports or (None, None) if none found.
        """
        # Get output ports
        out_response = requests.get(f"{test_env.base_url}/api/v1/links/ports/output")
        output_ports = out_response.json()["ports"]
        
        # Get input ports
        in_response = requests.get(f"{test_env.base_url}/api/v1/links/ports/input")
        input_ports = in_response.json()["ports"]
        
        # Get existing links to avoid conflicts
        links_response = requests.get(f"{test_env.base_url}/api/v1/links")
        existing_links = links_response.json()["links"]
        existing_pairs = {(l["output_port_name"], l["input_port_name"]) for l in existing_links}
        
        def is_midi_port(port):
            """Check if a port is a MIDI port (incompatible with audio)."""
            name = port["name"].lower()
            return "midi" in name or "bluez_midi" in name
        
        def is_audio_port(port):
            """Check if a port is an audio port (has channel indicators)."""
            name = port["port_name"].lower()
            # Common audio channel names
            return any(ch in name for ch in ["_fl", "_fr", "_fc", "_lfe", "_rl", "_rr", 
                                              "playback", "capture", "output", "input"])
        
        # Find a pair that's not already linked
        # Prefer ports from different nodes and matching channels (FL to FL, etc.)
        for out_port in output_ports:
            # Skip monitor ports
            if "monitor" in out_port["name"].lower():
                continue
            # Skip MIDI ports
            if is_midi_port(out_port):
                continue
            # Prefer audio ports
            if not is_audio_port(out_port):
                continue
                
            for in_port in input_ports:
                # Skip if same node
                if out_port["node_name"] == in_port["node_name"]:
                    continue
                # Skip if already linked
                if (out_port["name"], in_port["name"]) in existing_pairs:
                    continue
                # Skip MIDI ports
                if is_midi_port(in_port):
                    continue
                # Prefer audio ports
                if not is_audio_port(in_port):
                    continue
                return out_port, in_port
        
        return None, None
    
    def test_create_link_by_name(self, test_env):
        """Test creating a link by port name and verify with pw-link"""
        output_port, input_port = self.find_linkable_ports(test_env)
        
        if output_port is None:
            pytest.skip("No suitable ports found for linking test")
        
        output_name = output_port["name"]
        input_name = input_port["name"]
        
        # Create the link
        response = requests.post(
            f"{test_env.base_url}/api/v1/links",
            json={"output": output_name, "input": input_name}
        )
        assert response.status_code == 200, f"Failed to create link: {response.text}"
        
        data = response.json()
        assert data["status"] == "ok"
        
        # Verify link exists via API
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.status_code == 200
        assert exists_response.json()["exists"] == True
        
        # Clean up - remove the link
        link_id = exists_response.json().get("link_id")
        if link_id:
            requests.delete(f"{test_env.base_url}/api/v1/links/{link_id}")
    
    def test_create_and_remove_link_by_id(self, test_env):
        """Test creating a link by port ID and removing by link ID"""
        output_port, input_port = self.find_linkable_ports(test_env)
        
        if output_port is None:
            pytest.skip("No suitable ports found for linking test")
        
        output_id = output_port["id"]
        input_id = input_port["id"]
        output_name = output_port["name"]
        input_name = input_port["name"]
        
        # Create the link using IDs
        response = requests.post(
            f"{test_env.base_url}/api/v1/links",
            json={"output": str(output_id), "input": str(input_id)}
        )
        assert response.status_code == 200, f"Failed to create link: {response.text}"
        
        # Verify link exists via API
        time.sleep(0.1)  # Give PipeWire a moment to create the link
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == True, \
            f"Link {output_name} -> {input_name} not found via API after creation"
        
        # Get the link ID
        link_id = exists_response.json().get("link_id")
        assert link_id is not None, "Link ID not returned"
        
        # Remove the link by ID
        remove_response = requests.delete(f"{test_env.base_url}/api/v1/links/{link_id}")
        assert remove_response.status_code == 200, f"Failed to remove link: {remove_response.text}"
        
        # Verify link is gone via API
        time.sleep(0.1)  # Give PipeWire a moment
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == False, \
            f"Link {output_name} -> {input_name} still exists via API after removal"
    
    def test_remove_link_by_name(self, test_env):
        """Test creating and removing a link using port names"""
        output_port, input_port = self.find_linkable_ports(test_env)
        
        if output_port is None:
            pytest.skip("No suitable ports found for linking test")
        
        output_name = output_port["name"]
        input_name = input_port["name"]
        
        # Create the link
        response = requests.post(
            f"{test_env.base_url}/api/v1/links",
            json={"output": output_name, "input": input_name}
        )
        assert response.status_code == 200
        
        # Verify link exists via API
        time.sleep(0.1)
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == True
        
        # Remove the link by name
        remove_response = requests.delete(
            f"{test_env.base_url}/api/v1/links/by-name",
            json={"output": output_name, "input": input_name}
        )
        assert remove_response.status_code == 200
        
        # Verify link is gone via API
        time.sleep(0.1)
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == False, \
            f"Link still exists after removal by name"
    
    def test_link_round_trip(self, test_env):
        """Full round trip: create link, verify in API, remove, verify gone"""
        output_port, input_port = self.find_linkable_ports(test_env)
        
        if output_port is None:
            pytest.skip("No suitable ports found for linking test")
        
        output_name = output_port["name"]
        input_name = input_port["name"]
        
        # 1. Verify link doesn't exist initially via API
        initial_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        if initial_response.json()["exists"]:
            pytest.skip(f"Link {output_name} -> {input_name} already exists")
        
        # 2. Create the link via API
        create_response = requests.post(
            f"{test_env.base_url}/api/v1/links",
            json={"output": output_name, "input": input_name}
        )
        assert create_response.status_code == 200
        
        # 3. Verify link exists in API
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == True
        link_id = exists_response.json()["link_id"]
        
        # 4. Verify link appears in list
        list_response = requests.get(f"{test_env.base_url}/api/v1/links")
        links = list_response.json()["links"]
        found = any(l["output_port_name"] == output_name and l["input_port_name"] == input_name 
                   for l in links)
        assert found, "Link not found in links list"
        
        # 5. Remove the link
        remove_response = requests.delete(f"{test_env.base_url}/api/v1/links/{link_id}")
        assert remove_response.status_code == 200
        
        # 6. Verify link gone from API
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == False


class TestLinkExists:
    """Tests for the link exists endpoint"""
    
    def test_check_link_exists_returns_200(self, test_env):
        """Test that /api/v1/links/exists returns 200 OK"""
        response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": "dummy:port", "input": "other:port"}
        )
        assert response.status_code == 200
    
    def test_check_link_exists_structure(self, test_env):
        """Test that response has correct structure"""
        response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": "dummy:port", "input": "other:port"}
        )
        data = response.json()
        assert "exists" in data
        assert isinstance(data["exists"], bool)
    
    def test_nonexistent_link_returns_false(self, test_env):
        """Test that checking a non-existent link returns false"""
        response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": "nonexistent:port_FL", "input": "also_nonexistent:port_FL"}
        )
        data = response.json()
        assert data["exists"] == False
