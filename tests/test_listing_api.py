"""
Tests for the PipeWire API listing endpoints (/api/v1/ls/*)

These tests verify the listing endpoints work correctly and return
properly formatted JSON responses.
"""

import pytest
import requests
import subprocess
import time
import os
import socket


def find_free_port():
    """Find a free port to use for the test server"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(('', 0))
        s.listen(1)
        port = s.getsockname()[1]
    return port


class ListingTestEnvironment:
    """Test environment for listing API tests"""
    
    def __init__(self):
        self.port = find_free_port()
        self.base_url = f"http://127.0.0.1:{self.port}"
        self.server_process = None
        
    def start(self):
        """Start the API server"""
        build_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        binary_path = os.path.join(build_dir, "target", "release", "pipewire-api")
        
        # Build if needed
        subprocess.run(
            ["cargo", "build", "--release", "--bin", "pipewire-api"],
            cwd=build_dir,
            check=True,
            capture_output=True
        )
        
        # Start server
        self.server_process = subprocess.Popen(
            [binary_path, "-p", str(self.port)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=build_dir
        )
        
        # Wait for server to be ready
        for _ in range(30):
            try:
                response = requests.get(f"{self.base_url}/api/v1/ls", timeout=1)
                if response.status_code == 200:
                    return True
            except requests.exceptions.ConnectionError:
                pass
            time.sleep(0.2)
        
        return False
    
    def stop(self):
        """Stop the API server"""
        if self.server_process:
            self.server_process.terminate()
            try:
                self.server_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.server_process.kill()
            self.server_process = None


@pytest.fixture(scope="module")
def test_env():
    """Fixture providing a test environment with running server"""
    env = ListingTestEnvironment()
    if not env.start():
        pytest.skip("Could not start API server")
    yield env
    env.stop()


class TestListAll:
    """Tests for GET /api/v1/ls"""
    
    def test_list_all_returns_200(self, test_env):
        """Test that /api/v1/ls returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        assert response.status_code == 200
    
    def test_list_all_returns_json(self, test_env):
        """Test that /api/v1/ls returns valid JSON"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert isinstance(data, dict)
    
    def test_list_all_has_objects_array(self, test_env):
        """Test that response has 'objects' array"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert "objects" in data
        assert isinstance(data["objects"], list)
    
    def test_list_all_objects_have_required_fields(self, test_env):
        """Test that each object has id, name, and type fields"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        
        for obj in data["objects"]:
            assert "id" in obj, "Object missing 'id' field"
            assert "name" in obj, "Object missing 'name' field"
            assert "type" in obj, "Object missing 'type' field"
    
    def test_list_all_id_is_integer(self, test_env):
        """Test that object IDs are integers"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        
        for obj in data["objects"]:
            assert isinstance(obj["id"], int), f"ID should be int, got {type(obj['id'])}"
    
    def test_list_all_has_multiple_types(self, test_env):
        """Test that the listing includes multiple object types"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        
        types = set(obj["type"] for obj in data["objects"])
        # Should have at least nodes and modules
        assert len(types) >= 2, f"Expected multiple types, got: {types}"


class TestListNodes:
    """Tests for GET /api/v1/ls/nodes"""
    
    def test_list_nodes_returns_200(self, test_env):
        """Test that /api/v1/ls/nodes returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/nodes")
        assert response.status_code == 200
    
    def test_list_nodes_only_returns_nodes(self, test_env):
        """Test that /api/v1/ls/nodes only returns node objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/nodes")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "node", f"Expected type 'node', got '{obj['type']}'"
    
    def test_list_nodes_subset_of_all(self, test_env):
        """Test that nodes are a subset of all objects"""
        all_response = requests.get(f"{test_env.base_url}/api/v1/ls")
        nodes_response = requests.get(f"{test_env.base_url}/api/v1/ls/nodes")
        
        all_data = all_response.json()
        nodes_data = nodes_response.json()
        
        all_node_ids = {obj["id"] for obj in all_data["objects"] if obj["type"] == "node"}
        node_ids = {obj["id"] for obj in nodes_data["objects"]}
        
        assert node_ids == all_node_ids


class TestListDevices:
    """Tests for GET /api/v1/ls/devices"""
    
    def test_list_devices_returns_200(self, test_env):
        """Test that /api/v1/ls/devices returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/devices")
        assert response.status_code == 200
    
    def test_list_devices_only_returns_devices(self, test_env):
        """Test that /api/v1/ls/devices only returns device objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/devices")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "device", f"Expected type 'device', got '{obj['type']}'"


class TestListPorts:
    """Tests for GET /api/v1/ls/ports"""
    
    def test_list_ports_returns_200(self, test_env):
        """Test that /api/v1/ls/ports returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/ports")
        assert response.status_code == 200
    
    def test_list_ports_only_returns_ports(self, test_env):
        """Test that /api/v1/ls/ports only returns port objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/ports")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "port", f"Expected type 'port', got '{obj['type']}'"


class TestListModules:
    """Tests for GET /api/v1/ls/modules"""
    
    def test_list_modules_returns_200(self, test_env):
        """Test that /api/v1/ls/modules returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/modules")
        assert response.status_code == 200
    
    def test_list_modules_only_returns_modules(self, test_env):
        """Test that /api/v1/ls/modules only returns module objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/modules")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "module", f"Expected type 'module', got '{obj['type']}'"
    
    def test_list_modules_has_pipewire_modules(self, test_env):
        """Test that some standard PipeWire modules are present"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/modules")
        data = response.json()
        
        names = [obj["name"] for obj in data["objects"]]
        # Should have at least the rt module
        assert any("libpipewire-module" in name for name in names), \
            f"Expected PipeWire modules, got: {names[:5]}..."


class TestListFactories:
    """Tests for GET /api/v1/ls/factories"""
    
    def test_list_factories_returns_200(self, test_env):
        """Test that /api/v1/ls/factories returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/factories")
        assert response.status_code == 200
    
    def test_list_factories_only_returns_factories(self, test_env):
        """Test that /api/v1/ls/factories only returns factory objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/factories")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "factory", f"Expected type 'factory', got '{obj['type']}'"


class TestListClients:
    """Tests for GET /api/v1/ls/clients"""
    
    def test_list_clients_returns_200(self, test_env):
        """Test that /api/v1/ls/clients returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/clients")
        assert response.status_code == 200
    
    def test_list_clients_only_returns_clients(self, test_env):
        """Test that /api/v1/ls/clients only returns client objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/clients")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "client", f"Expected type 'client', got '{obj['type']}'"


class TestListLinks:
    """Tests for GET /api/v1/ls/links"""
    
    def test_list_links_returns_200(self, test_env):
        """Test that /api/v1/ls/links returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/links")
        assert response.status_code == 200
    
    def test_list_links_only_returns_links(self, test_env):
        """Test that /api/v1/ls/links only returns link objects"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/links")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["type"] == "link", f"Expected type 'link', got '{obj['type']}'"
    
    def test_list_links_name_shows_connection(self, test_env):
        """Test that link names show connection info (node:port -> node:port)"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls/links")
        data = response.json()
        
        if data["objects"]:  # Only test if there are links
            for obj in data["objects"]:
                # Link names should contain "->" showing the connection
                assert "->" in obj["name"] or obj["name"].isdigit(), \
                    f"Expected link name to show connection, got: {obj['name']}"


class TestObjectIdUniqueness:
    """Tests for object ID consistency"""
    
    def test_all_ids_unique(self, test_env):
        """Test that all object IDs are unique"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        
        ids = [obj["id"] for obj in data["objects"]]
        assert len(ids) == len(set(ids)), "Duplicate IDs found"
    
    def test_ids_are_positive(self, test_env):
        """Test that all object IDs are non-negative"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        
        for obj in data["objects"]:
            assert obj["id"] >= 0, f"Negative ID found: {obj['id']}"


class TestResponseFormat:
    """Tests for response format consistency"""
    
    def test_content_type_is_json(self, test_env):
        """Test that response Content-Type is application/json"""
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        assert "application/json" in response.headers.get("Content-Type", "")
    
    def test_empty_list_format(self, test_env):
        """Test that empty results still return proper format"""
        # Use a type filter that might return empty results
        response = requests.get(f"{test_env.base_url}/api/v1/ls/links")
        data = response.json()
        
        assert "objects" in data
        assert isinstance(data["objects"], list)


class TestGetObjectById:
    """Tests for GET /api/v1/objects/:id"""
    
    def test_get_object_by_id_returns_200(self, test_env):
        """Test that getting an existing object returns 200 OK"""
        # First get a list of objects to find a valid ID
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert len(data["objects"]) > 0, "No objects found"
        
        obj_id = data["objects"][0]["id"]
        response = requests.get(f"{test_env.base_url}/api/v1/objects/{obj_id}")
        assert response.status_code == 200
    
    def test_get_object_by_id_returns_correct_object(self, test_env):
        """Test that the returned object has the correct ID"""
        # Get a list of objects to find a valid ID
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert len(data["objects"]) > 0, "No objects found"
        
        obj_id = data["objects"][0]["id"]
        response = requests.get(f"{test_env.base_url}/api/v1/objects/{obj_id}")
        obj = response.json()
        
        assert obj["id"] == obj_id
    
    def test_get_object_by_id_has_required_fields(self, test_env):
        """Test that the returned object has all required fields"""
        # Get a list of objects to find a valid ID
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert len(data["objects"]) > 0, "No objects found"
        
        obj_id = data["objects"][0]["id"]
        response = requests.get(f"{test_env.base_url}/api/v1/objects/{obj_id}")
        obj = response.json()
        
        assert "id" in obj
        assert "name" in obj
        assert "type" in obj
    
    def test_get_object_by_invalid_id_returns_404(self, test_env):
        """Test that getting a non-existent object returns 404"""
        response = requests.get(f"{test_env.base_url}/api/v1/objects/999999")
        assert response.status_code == 404
    
    def test_get_object_matches_list(self, test_env):
        """Test that getting an object by ID matches the list data"""
        # Get all objects
        response = requests.get(f"{test_env.base_url}/api/v1/ls")
        data = response.json()
        assert len(data["objects"]) > 0, "No objects found"
        
        # Pick an object and verify it matches
        list_obj = data["objects"][0]
        response = requests.get(f"{test_env.base_url}/api/v1/objects/{list_obj['id']}")
        single_obj = response.json()
        
        assert single_obj["id"] == list_obj["id"]
        assert single_obj["name"] == list_obj["name"]
        assert single_obj["type"] == list_obj["type"]


class TestCacheRefresh:
    """Tests for POST /api/v1/cache/refresh"""
    
    def test_refresh_cache_returns_200(self, test_env):
        """Test that refreshing cache returns 200 OK"""
        response = requests.post(f"{test_env.base_url}/api/v1/cache/refresh")
        assert response.status_code == 200
    
    def test_refresh_cache_returns_status(self, test_env):
        """Test that refresh response includes status"""
        response = requests.post(f"{test_env.base_url}/api/v1/cache/refresh")
        data = response.json()
        
        assert "status" in data
        assert data["status"] == "ok"
    
    def test_refresh_cache_returns_object_count(self, test_env):
        """Test that refresh response includes object count"""
        response = requests.post(f"{test_env.base_url}/api/v1/cache/refresh")
        data = response.json()
        
        assert "object_count" in data
        assert isinstance(data["object_count"], int)
        assert data["object_count"] >= 0
    
    def test_refresh_cache_object_count_matches_ls(self, test_env):
        """Test that cache object count matches ls endpoint"""
        # Refresh the cache
        refresh_response = requests.post(f"{test_env.base_url}/api/v1/cache/refresh")
        refresh_data = refresh_response.json()
        
        # Get all objects
        ls_response = requests.get(f"{test_env.base_url}/api/v1/ls")
        ls_data = ls_response.json()
        
        assert refresh_data["object_count"] == len(ls_data["objects"])
