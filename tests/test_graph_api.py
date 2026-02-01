"""
Tests for the PipeWire API graph endpoints (/api/v1/graph, /api/v1/graph/png)

These tests verify the graph visualization endpoints work correctly and return
properly formatted DOT and PNG responses.
"""

import pytest
import requests
import re


# Note: test_env fixture is provided by conftest.py (session-scoped)


class TestGraphDot:
    """Tests for GET /api/v1/graph (DOT format)"""
    
    def test_graph_dot_returns_200(self, test_env):
        """Test that /api/v1/graph returns 200 OK"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        assert response.status_code == 200
    
    def test_graph_dot_content_type(self, test_env):
        """Test that /api/v1/graph returns correct content type"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        content_type = response.headers.get("content-type", "")
        assert "text/vnd.graphviz" in content_type or "text/plain" in content_type
    
    def test_graph_dot_is_valid_dot(self, test_env):
        """Test that response is valid DOT format"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Check DOT structure
        assert dot.strip().startswith("digraph"), "DOT should start with 'digraph'"
        assert "}" in dot, "DOT should have closing brace"
        assert "rankdir=" in dot, "DOT should have rankdir directive"
    
    def test_graph_dot_has_nodes(self, test_env):
        """Test that graph contains node definitions"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Should have either regular nodes or chain nodes
        has_nodes = "node_" in dot or "chain_" in dot
        assert has_nodes, "Graph should contain node or chain definitions"
    
    def test_graph_dot_has_color_coding(self, test_env):
        """Test that graph uses color coding for different node types"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Check for color attributes (lightgreen only present if sources exist)
        assert "lightblue" in dot or "lightyellow" in dot, "Graph should use colors for nodes"
        # Filter chains use lightyellow
        if "chain_" in dot:
            assert "lightyellow" in dot, "Graph should use lightyellow for filter chains"
    
    def test_graph_dot_filter_chains_combined(self, test_env):
        """Test that filter-chains are combined into single nodes"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # If there are chain nodes, they should have combined IDs
        chain_pattern = r'chain_\d+ \[label="[^"]+\\nID: (\d+)/(\d+)"'
        chain_matches = re.findall(chain_pattern, dot)
        
        for match in chain_matches:
            input_id, output_id = int(match[0]), int(match[1])
            assert input_id != output_id, "Filter-chain should combine two different node IDs"
    
    def test_graph_dot_no_internal_chain_links(self, test_env):
        """Test that internal filter-chain links are not shown"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Find all chain IDs
        chain_ids = re.findall(r'chain_(\d+) \[', dot)
        
        # Check that no chain links to itself
        for chain_id in chain_ids:
            self_link = f"chain_{chain_id} -> chain_{chain_id}"
            assert self_link not in dot, f"Chain {chain_id} should not have self-link"
    
    def test_graph_dot_has_device_cluster(self, test_env):
        """Test that devices are in their own cluster (if any devices exist)"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Check for device cluster if any devices
        if "dev_" in dot:
            assert "cluster_devices" in dot, "Devices should be in cluster_devices subgraph"
            assert 'label="Devices"' in dot, "Device cluster should have 'Devices' label"


class TestGraphPng:
    """Tests for GET /api/v1/graph/png (PNG format)"""
    
    def test_graph_png_returns_200_or_404(self, test_env):
        """Test that /api/v1/graph/png returns 200 OK or 404 if graphviz not installed"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph/png")
        assert response.status_code in [200, 404], f"Expected 200 or 404, got {response.status_code}"
    
    def test_graph_png_content_type(self, test_env):
        """Test that /api/v1/graph/png returns correct content type when successful"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph/png")
        
        if response.status_code == 200:
            content_type = response.headers.get("content-type", "")
            assert "image/png" in content_type, f"Expected image/png, got {content_type}"
    
    def test_graph_png_is_valid_png(self, test_env):
        """Test that response is valid PNG data when successful"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph/png")
        
        if response.status_code == 200:
            # PNG magic bytes
            png_signature = b'\x89PNG\r\n\x1a\n'
            assert response.content[:8] == png_signature, "Response should be valid PNG"
    
    def test_graph_png_has_reasonable_size(self, test_env):
        """Test that PNG has reasonable size (not empty, not too small)"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph/png")
        
        if response.status_code == 200:
            # Should be at least 1KB (a tiny graph would still be a few KB)
            assert len(response.content) > 1000, f"PNG seems too small: {len(response.content)} bytes"
    
    def test_graph_png_404_message(self, test_env):
        """Test that 404 response has informative message"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph/png")
        
        if response.status_code == 404:
            # Should indicate graphviz is not found
            text = response.text.lower()
            assert "graphviz" in text or "not found" in text, "404 should mention graphviz"


class TestGraphEndpointListing:
    """Tests for graph endpoints in API listing"""
    
    def test_graph_endpoints_in_api_listing(self, test_env):
        """Test that graph endpoints are listed in /api/v1"""
        response = requests.get(f"{test_env.base_url}/api/v1")
        
        if response.status_code == 200:
            data = response.json()
            endpoints = data.get("endpoints", [])
            paths = [e.get("path", "") for e in endpoints]
            
            assert "/api/v1/graph" in paths, "Graph DOT endpoint should be listed"
            assert "/api/v1/graph/png" in paths, "Graph PNG endpoint should be listed"


class TestGraphContent:
    """Tests for graph content and structure"""
    
    def test_graph_nodes_have_ids(self, test_env):
        """Test that all nodes in graph have ID labels"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Find all node labels
        node_labels = re.findall(r'\[label="[^"]+\\nID: ([^"]+)"', dot)
        
        # Each node/chain should have an ID
        for node_id in node_labels:
            # ID should be numeric or numeric/numeric for chains
            assert re.match(r'^\d+(/\d+)?$', node_id), f"Invalid node ID format: {node_id}"
    
    def test_graph_links_reference_valid_nodes(self, test_env):
        """Test that all links reference existing nodes"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text
        
        # Extract all defined node names
        defined_nodes = set()
        defined_nodes.update(re.findall(r'(node_\d+) \[', dot))
        defined_nodes.update(re.findall(r'(chain_\d+) \[', dot))
        defined_nodes.update(re.findall(r'(dev_\d+) \[', dot))
        # Also add legend nodes
        defined_nodes.update(re.findall(r'(legend_\w+) \[', dot))
        
        # Extract all link references
        links = re.findall(r'(\w+_\d+) -> (\w+_\d+)', dot)
        
        for source, target in links:
            assert source in defined_nodes, f"Link source '{source}' not defined"
            assert target in defined_nodes, f"Link target '{target}' not defined"
    
    def test_graph_excludes_midi_nodes(self, test_env):
        """Test that MIDI nodes are excluded from the graph"""
        response = requests.get(f"{test_env.base_url}/api/v1/graph")
        dot = response.text.lower()
        
        # MIDI should not appear in node labels
        # (it might appear in the graph name but not as actual audio nodes)
        midi_node_pattern = r'node_\d+.*midi'
        assert not re.search(midi_node_pattern, dot), "MIDI nodes should be excluded"
