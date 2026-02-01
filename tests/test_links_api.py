"""
Tests for the Links API endpoints.

These tests verify link creation, listing, and removal using the API,
and verify results using the pw-link command line tool.
"""

import os
import subprocess
import socket
import time
import signal
import pytest
import requests


def find_free_port():
    """Find a free port to use for testing."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('', 0))
        s.listen(1)
        port = s.getsockname()[1]
    return port


class _TestEnv:
    """Test environment with API server (internal, not a test class)."""
    def __init__(self, base_url, process):
        self.base_url = base_url
        self.process = process


@pytest.fixture(scope="module")
def test_env():
    """Start the API server for testing."""
    port = find_free_port()
    base_url = f"http://127.0.0.1:{port}"
    
    # Build the server if not already built
    build_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    # Start the API server
    server_path = os.path.join(build_dir, "target/release/pipewire-api")
    process = subprocess.Popen(
        [server_path, "--port", str(port), "--localhost"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=os.setsid
    )
    
    # Wait for server to start
    max_retries = 30
    for i in range(max_retries):
        try:
            requests.get(f"{base_url}/api/v1/ls", timeout=1)
            break
        except requests.exceptions.ConnectionError:
            time.sleep(0.1)
    else:
        process.terminate()
        raise RuntimeError("Server failed to start")
    
    env = _TestEnv(base_url, process)
    yield env
    
    # Cleanup
    os.killpg(os.getpgid(process.pid), signal.SIGTERM)
    process.wait()


def run_pw_link(*args):
    """Run pw-link command and return output."""
    result = subprocess.run(
        ["pw-link"] + list(args),
        capture_output=True,
        text=True
    )
    return result.returncode, result.stdout, result.stderr


def get_pw_link_list():
    """Get current links from pw-link -l -I."""
    _, stdout, _ = run_pw_link("-l", "-I")
    return stdout


def link_exists_in_pw_link(output_port, input_port):
    """Check if a link exists using pw-link output."""
    stdout = get_pw_link_list()
    # Looking for pattern: "  92   |->   82 speakereq2x2:playback_FL"
    # After the output port "  90 effect_output.proc:output_FL"
    lines = stdout.split('\n')
    found_output = False
    for line in lines:
        if output_port in line and '|->' not in line and '|<-' not in line:
            found_output = True
        elif found_output and '|->' in line and input_port in line:
            return True
        elif found_output and '|->' not in line and '|<-' not in line and line.strip():
            found_output = False
    return False


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
        """Find an output and input port that can be linked for testing."""
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
        
        # Find a pair that's not already linked
        # Prefer ports from different nodes and matching channels (FL to FL, etc.)
        for out_port in output_ports:
            for in_port in input_ports:
                # Skip if same node
                if out_port["node_name"] == in_port["node_name"]:
                    continue
                # Skip if already linked
                if (out_port["name"], in_port["name"]) in existing_pairs:
                    continue
                # Skip monitor ports
                if "monitor" in out_port["name"].lower():
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
        
        # Verify link exists via pw-link command
        assert link_exists_in_pw_link(output_name, input_name), \
            f"Link {output_name} -> {input_name} not found in pw-link output"
        
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
        
        # Verify link exists via pw-link
        time.sleep(0.1)  # Give PipeWire a moment to create the link
        assert link_exists_in_pw_link(output_name, input_name), \
            f"Link {output_name} -> {input_name} not found in pw-link output after creation"
        
        # Get the link ID
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        link_id = exists_response.json().get("link_id")
        assert link_id is not None, "Link ID not returned"
        
        # Remove the link by ID
        remove_response = requests.delete(f"{test_env.base_url}/api/v1/links/{link_id}")
        assert remove_response.status_code == 200, f"Failed to remove link: {remove_response.text}"
        
        # Verify link is gone via pw-link
        time.sleep(0.1)  # Give PipeWire a moment
        assert not link_exists_in_pw_link(output_name, input_name), \
            f"Link {output_name} -> {input_name} still exists in pw-link output after removal"
    
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
        
        # Verify link exists
        time.sleep(0.1)
        assert link_exists_in_pw_link(output_name, input_name)
        
        # Remove the link by name
        remove_response = requests.delete(
            f"{test_env.base_url}/api/v1/links/by-name",
            json={"output": output_name, "input": input_name}
        )
        assert remove_response.status_code == 200
        
        # Verify link is gone via pw-link
        time.sleep(0.1)
        assert not link_exists_in_pw_link(output_name, input_name), \
            f"Link still exists after removal by name"
    
    def test_link_round_trip(self, test_env):
        """Full round trip: create link, verify in API and pw-link, remove, verify gone"""
        output_port, input_port = self.find_linkable_ports(test_env)
        
        if output_port is None:
            pytest.skip("No suitable ports found for linking test")
        
        output_name = output_port["name"]
        input_name = input_port["name"]
        
        # 1. Verify link doesn't exist initially (in pw-link)
        initial_exists = link_exists_in_pw_link(output_name, input_name)
        # If it exists, skip this test
        if initial_exists:
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
        
        # 4. Verify link exists in pw-link
        time.sleep(0.1)
        assert link_exists_in_pw_link(output_name, input_name), \
            "Link not visible in pw-link after creation"
        
        # 5. Verify link appears in list
        list_response = requests.get(f"{test_env.base_url}/api/v1/links")
        links = list_response.json()["links"]
        found = any(l["output_port_name"] == output_name and l["input_port_name"] == input_name 
                   for l in links)
        assert found, "Link not found in links list"
        
        # 6. Remove the link
        remove_response = requests.delete(f"{test_env.base_url}/api/v1/links/{link_id}")
        assert remove_response.status_code == 200
        
        # 7. Verify link gone from API
        exists_response = requests.get(
            f"{test_env.base_url}/api/v1/links/exists",
            params={"output": output_name, "input": input_name}
        )
        assert exists_response.json()["exists"] == False
        
        # 8. Verify link gone from pw-link
        time.sleep(0.1)
        assert not link_exists_in_pw_link(output_name, input_name), \
            "Link still visible in pw-link after removal"


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
