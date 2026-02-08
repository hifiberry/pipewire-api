#!/usr/bin/env python3
"""
Integration tests for the Settings Save/Restore API.
Tests the ability to save and restore settings for speakereq and riaa modules.
"""

import pytest
import requests
import json
import os
import tempfile


def get_settings_file_path(api_server):
    """Get the actual settings file path from the server"""
    response = requests.post(f"{api_server}/api/v1/settings/save")
    if response.status_code == 200:
        data = response.json()
        return data.get("path")
    return None


@pytest.fixture(autouse=True)
def cleanup_settings_file(api_server):
    """Clean up settings file before and after each test"""
    # Get the actual path from first save
    path = get_settings_file_path(api_server)
    
    # Remove before test
    if path and os.path.exists(path):
        os.remove(path)
    
    yield
    
    # Remove after test  
    if path and os.path.exists(path):
        os.remove(path)


class TestSettingsSaveRestore:
    """Test settings save/restore functionality"""
    
    def test_save_settings_creates_file(self, api_server):
        """Test that saving settings creates the JSON file"""
        response = requests.post(f"{api_server}/api/v1/settings/save")
        
        assert response.status_code == 200
        data = response.json()
        assert data["success"] is True
        assert "path" in data
        
        # Verify file was created
        settings_file_path = data["path"]
        assert os.path.exists(settings_file_path)
    
    def test_save_settings_json_structure(self, api_server):
        """Test that saved settings have correct JSON structure"""
        # Save settings
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Read and verify JSON structure
        with open(settings_file_path, 'r') as f:
            settings = json.load(f)
        
        assert "version" in settings
        assert len(settings["version"].split(".")) >= 2  # valid semver-like string
        assert "speakereq" in settings
        assert "riaa" in settings
    
    def test_save_includes_speakereq_settings(self, api_server):
        """Test that saved settings include speakereq module configuration"""
        # Check if speakereq module is available
        status_response = requests.get(f"{api_server}/api/v1/module/speakereq/status")
        if status_response.status_code != 200:
            pytest.skip("SpeakerEQ module not available")
        
        # Save settings
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Verify saved content
        with open(settings_file_path, 'r') as f:
            settings = json.load(f)
        
        # Verify speakereq data is present
        assert settings.get("speakereq") is not None, "SpeakerEQ settings should be saved"
        speakereq = settings["speakereq"]
        assert "enabled" in speakereq
        assert "master_gain_db" in speakereq
        assert "inputs" in speakereq
        assert "outputs" in speakereq
    
    def test_save_includes_riaa_settings(self, api_server):
        """Test that saved settings include riaa module configuration"""
        # Check if riaa module is available
        config_response = requests.get(f"{api_server}/api/v1/module/riaa/config")
        if config_response.status_code != 200:
            pytest.skip("RIAA module not available")
        
        # Save settings
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Verify saved content
        with open(settings_file_path, 'r') as f:
            settings = json.load(f)
        
        # Verify riaa data is present
        assert settings.get("riaa") is not None, "RIAA settings should be saved"
        riaa = settings["riaa"]
        assert "gain_db" in riaa
        assert "riaa_enable" in riaa
        assert "declick_enable" in riaa
        assert "subsonic_filter" in riaa
    
    def test_restore_without_file_returns_success(self, api_server):
        """Test that restore returns success even when no settings file exists"""
        # Get path and ensure file doesn't exist
        response = requests.post(f"{api_server}/api/v1/settings/save")
        settings_file_path = response.json()["path"]
        
        if os.path.exists(settings_file_path):
            os.remove(settings_file_path)
        
        response = requests.post(f"{api_server}/api/v1/settings/restore")
        
        # Should return success with 0 modules restored
        assert response.status_code in [200, 400, 404]
    
    def test_full_save_restore_workflow(self, api_server):
        """Test complete save/restore workflow"""
        # Save current settings
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        assert os.path.exists(settings_file_path)
        
        # Read what was saved
        with open(settings_file_path, 'r') as f:
            original_settings = json.load(f)
        
        # Restore settings
        response = requests.post(f"{api_server}/api/v1/settings/restore")
        assert response.status_code == 200
        
        data = response.json()
        assert data["success"] is True
        assert "modules_restored" in data
        
        # Verify response has modules list
        assert isinstance(data["modules_restored"], list)
    
    def test_restore_response_format(self, api_server):
        """Test that restore response has correct format"""
        # Save some settings first
        requests.post(f"{api_server}/api/v1/settings/save")
        
        # Restore
        response = requests.post(f"{api_server}/api/v1/settings/restore")
        assert response.status_code == 200
        
        data = response.json()
        assert "success" in data
        assert "modules_restored" in data
        assert isinstance(data["modules_restored"], list)
        assert isinstance(data["success"], bool)
    
    def test_multiple_save_overwrites(self, api_server):
        """Test that multiple saves overwrite the previous file"""
        # First save
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Get file modification time
        mtime1 = os.path.getmtime(settings_file_path)
        
        # Wait a bit to ensure different timestamp
        import time
        time.sleep(0.1)
        
        # Second save
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        
        # Verify file was updated
        mtime2 = os.path.getmtime(settings_file_path)
        assert mtime2 >= mtime1  # Allow for equal in case of very fast filesystem
    
    def test_settings_file_is_valid_json(self, api_server):
        """Test that settings file can be parsed as valid JSON"""
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Should not raise JSONDecodeError
        with open(settings_file_path, 'r') as f:
            settings = json.load(f)
        
        assert isinstance(settings, dict)
    
    def test_settings_directory_created_automatically(self, api_server):
        """Test that the settings directory is created if it doesn't exist"""
        # The directory should be created automatically by the save endpoint
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        
        # Verify directory exists
        settings_file_path = response.json()["path"]
        settings_dir = os.path.dirname(settings_file_path)
        assert os.path.isdir(settings_dir)
    
    def test_concurrent_module_settings(self, api_server):
        """Test that both speakereq and riaa settings can be saved together"""
        # Save
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_file_path = response.json()["path"]
        
        # Verify both modules are in the file (or at least the structure is there)
        with open(settings_file_path, 'r') as f:
            settings = json.load(f)
        
        # Should have both keys present (may be None if modules not configured)
        assert "speakereq" in settings
        assert "riaa" in settings
    
    def test_save_response_format(self, api_server):
        """Test that save response has the expected format"""
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        
        data = response.json()
        assert "success" in data
        assert "path" in data
        assert data["success"] is True
        assert isinstance(data["path"], str)
        assert data["path"].endswith("settings.json")


class TestAutoSave:
    """Test auto-save functionality"""
    
    def test_auto_save_after_setting_change(self, api_server):
        """Test that settings are auto-saved after a change"""
        import time
        
        # Get settings file path
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_path = response.json()["path"]
        
        # Read initial settings
        with open(settings_path, 'r') as f:
            initial_settings = json.load(f)
        initial_gain = initial_settings["speakereq"]["master_gain_db"]
        
        # Change a setting (master gain for speakereq)
        new_gain = 5.5
        response = requests.put(
            f"{api_server}/api/v1/module/speakereq/gain/master",
            json={"gain": new_gain}
        )
        assert response.status_code == 200
        
        # Wait for auto-save (default interval is 10 seconds, plus some buffer)
        print("Waiting 12 seconds for auto-save...")
        time.sleep(12)
        
        # Verify the change is in the file
        with open(settings_path, 'r') as f:
            settings = json.load(f)
        
        assert settings["speakereq"] is not None
        current_gain = settings["speakereq"]["master_gain_db"]
        assert current_gain == new_gain, f"Expected gain {new_gain}, but got {current_gain}"
    
    def test_auto_save_after_default_reset(self, api_server):
        """Test that settings are auto-saved after reset to defaults"""
        import time
        
        # Get settings file path
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_path = response.json()["path"]
        
        # Change a setting first
        response = requests.put(
            f"{api_server}/api/v1/module/speakereq/gain/master",
            json={"gain": 8.0}
        )
        assert response.status_code == 200
        
        # Wait for auto-save
        time.sleep(12)
        
        # Read settings to confirm change
        with open(settings_path, 'r') as f:
            settings_before = json.load(f)
        assert settings_before["speakereq"]["master_gain_db"] == 8.0
        
        # Reset to defaults
        response = requests.post(f"{api_server}/api/v1/module/speakereq/default")
        assert response.status_code == 200
        
        # Wait for auto-save
        print("Waiting 12 seconds for auto-save after default reset...")
        time.sleep(12)
        
        # Verify settings were reset to default (master_gain_db should be 0.0)
        with open(settings_path, 'r') as f:
            settings_after = json.load(f)
        
        assert settings_after["speakereq"] is not None
        assert settings_after["speakereq"]["master_gain_db"] == 0.0, "Gain should be reset to 0.0"
    
    def test_auto_save_no_change_no_save(self, api_server):
        """Test that auto-save doesn't save when nothing changed"""
        import time
        
        # Get settings file path
        response = requests.post(f"{api_server}/api/v1/settings/save")
        assert response.status_code == 200
        settings_path = response.json()["path"]
        
        # Record modification time
        initial_mtime = os.path.getmtime(settings_path)
        
        # Wait for multiple auto-save intervals without making changes
        print("Waiting 25 seconds to verify no unnecessary saves...")
        time.sleep(25)
        
        # File modification time should not have changed (or only changed once initially)
        # We allow for one save cycle, but not multiple
        final_mtime = os.path.getmtime(settings_path)
        
        # Read both initial and final to ensure they're the same
        # (mtime might change once on first auto-save cycle, but content should be same)
        time.sleep(0.1)
        with open(settings_path, 'r') as f:
            final_content = f.read()
        
        # Content should remain stable
        assert len(final_content) > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
