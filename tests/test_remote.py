#!/usr/bin/env python3
"""
Remote test runner for PipeWire API.

This script runs the test suite against a remote server using HTTP requests only.
Tests that require local access (state files, PipeWire CLI tools) are automatically skipped.

Usage:
    # Run against a remote server
    python3 tests/test_remote.py http://192.168.11.136:2716
    
    # With verbose output
    python3 tests/test_remote.py http://192.168.11.136:2716 -v
    
    # Run specific test file
    python3 tests/test_remote.py http://192.168.11.136:2716 tests/test_speakereq.py
"""

import sys
import os
import subprocess


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        print("Error: Remote server URL required")
        print("Example: python3 tests/test_remote.py http://192.168.11.136:2716")
        sys.exit(1)
    
    remote_url = sys.argv[1]
    
    # Validate URL format
    if not remote_url.startswith("http://") and not remote_url.startswith("https://"):
        print(f"Error: Invalid URL format: {remote_url}")
        print("URL must start with http:// or https://")
        sys.exit(1)
    
    # Build pytest arguments
    pytest_args = [
        "python3", "-m", "pytest",
        "-m", "not local_only",  # Skip tests marked as local_only
    ]
    
    # Add any additional arguments (like -v, specific test files)
    pytest_args.extend(sys.argv[2:])
    
    # If no specific test files given, run all tests
    if not any(arg.startswith("tests/") or arg.endswith(".py") for arg in sys.argv[2:]):
        pytest_args.append("tests/")
    
    # Set environment variable for remote URL
    env = os.environ.copy()
    env["PIPEWIRE_API_REMOTE_URL"] = remote_url
    
    print(f"Running tests against remote server: {remote_url}")
    print(f"Command: {' '.join(pytest_args)}")
    print("-" * 60)
    sys.stdout.flush()
    
    # Run pytest
    result = subprocess.run(pytest_args, env=env)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
