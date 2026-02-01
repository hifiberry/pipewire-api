"""Utility functions for testing PipeWire parameter operations."""
import subprocess
import re
from typing import Optional, Union


def get_pipewire_param(node_id: int, param_name: str) -> Optional[Union[float, int, bool, str]]:
    """
    Get a parameter value directly from PipeWire using pw-cli.
    
    Args:
        node_id: The PipeWire node ID
        param_name: The full parameter name (e.g., "riaa:Gain (dB)")
    
    Returns:
        The parameter value (float, int, bool, or str) or None if not found
    """
    try:
        # Run pw-cli enum-params to get all parameters
        result = subprocess.run(
            ["pw-cli", "enum-params", str(node_id), "Props"],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if result.returncode != 0:
            print(f"pw-cli failed: {result.stderr}")
            return None
        
        output = result.stdout
        lines = output.split('\n')
        
        # Look for the parameter name followed by its value
        for i, line in enumerate(lines):
            if f'String "{param_name}"' in line:
                # The value should be on the next line
                if i + 1 < len(lines):
                    value_line = lines[i + 1].strip()
                    
                    # Parse Float value
                    float_match = re.match(r'Float\s+([-+]?[0-9]*\.?[0-9]+)', value_line)
                    if float_match:
                        return float(float_match.group(1))
                    
                    # Parse Int value
                    int_match = re.match(r'Int\s+([-+]?[0-9]+)', value_line)
                    if int_match:
                        return int(int_match.group(1))
                    
                    # Parse Bool value
                    bool_match = re.match(r'Bool\s+(true|false)', value_line)
                    if bool_match:
                        return bool_match.group(1) == 'true'
                    
                    # Parse String value
                    string_match = re.match(r'String\s+"(.+)"', value_line)
                    if string_match:
                        return string_match.group(1)
        
        return None
        
    except subprocess.TimeoutExpired:
        print(f"Timeout getting parameter {param_name} from node {node_id}")
        return None
    except Exception as e:
        print(f"Error getting parameter {param_name}: {e}")
        return None


def verify_param_set(node_id: int, param_name: str, expected_value: Union[float, int, bool, str], tolerance: float = 0.01) -> bool:
    """
    Verify that a parameter was actually set in PipeWire.
    
    Args:
        node_id: The PipeWire node ID
        param_name: The full parameter name (e.g., "riaa:Gain (dB)")
        expected_value: The expected value
        tolerance: For float values, the acceptable difference
    
    Returns:
        True if the parameter matches the expected value
    """
    actual_value = get_pipewire_param(node_id, param_name)
    
    if actual_value is None:
        print(f"Parameter {param_name} not found in PipeWire")
        return False
    
    if isinstance(expected_value, float) and isinstance(actual_value, float):
        match = abs(actual_value - expected_value) <= tolerance
        if not match:
            print(f"Parameter {param_name}: expected {expected_value}, got {actual_value} (tolerance: {tolerance})")
        return match
    else:
        match = actual_value == expected_value
        if not match:
            print(f"Parameter {param_name}: expected {expected_value}, got {actual_value}")
        return match


def set_pipewire_param(node_id: int, param_name: str, value: Union[float, int, bool, str]) -> bool:
    """
    Set a parameter directly in PipeWire using pw-cli.
    
    Args:
        node_id: The PipeWire node ID
        param_name: The full parameter name (e.g., "riaa:Gain (dB)")
        value: The value to set
    
    Returns:
        True if the parameter was set successfully
    """
    try:
        # Build the JSON for pw-cli set-param
        if isinstance(value, bool):
            json_value = "true" if value else "false"
        elif isinstance(value, str):
            json_value = f'"{value}"'
        else:
            json_value = str(value)
        
        json_str = f'{{"params": ["{param_name}", {json_value}]}}'
        
        result = subprocess.run(
            ["pw-cli", "set-param", str(node_id), "Props", json_str],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        if result.returncode != 0:
            print(f"pw-cli set-param failed: {result.stderr}")
            return False
        
        return True
        
    except subprocess.TimeoutExpired:
        print(f"Timeout setting parameter {param_name} on node {node_id}")
        return False
    except Exception as e:
        print(f"Error setting parameter {param_name}: {e}")
        return False
