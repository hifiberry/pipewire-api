#!/bin/bash

# Navigate to the script's directory
cd "$(dirname "$0")"

# Create virtual environment if it doesn't exist
if [ ! -d ".venv" ]; then
    echo "Creating virtual environment..."
    python3 -m venv .venv
fi

# Activate virtual environment
source .venv/bin/activate

# Install dependencies if needed
if ! pip show pytest &>/dev/null || ! pip show requests &>/dev/null; then
    echo "Installing test dependencies..."
    pip install -q pytest requests
fi

# Run tests
echo "Running tests..."
pytest tests/test_api.py -v

# Capture exit code
EXIT_CODE=$?

# Deactivate virtual environment
deactivate

exit $EXIT_CODE
