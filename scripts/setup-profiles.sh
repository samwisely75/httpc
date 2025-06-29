#!/bin/bash
# Setup script for webly - creates initial ~/.webly/profiles configuration

set -e

# Create webly config directory
CONFIG_DIR="$HOME/.webly"
PROFILES_FILE="$CONFIG_DIR/profiles"

# Create directory if it doesn't exist
if [ ! -d "$CONFIG_DIR" ]; then
    mkdir -p "$CONFIG_DIR"
    echo "Created webly config directory: $CONFIG_DIR"
fi

# Create initial profiles file if it doesn't exist
if [ ! -f "$PROFILES_FILE" ]; then
    cat > "$PROFILES_FILE" << 'EOF'
# Webly Profiles Configuration
# 
# This file contains profile definitions for the webly HTTP client.
# Each profile section defines connection settings and default headers.
#
# Example profiles:

[httpbin]
host = https://httpbin.org
@content-type = application/json
@user-agent = webly/0.1.6

[jsonplaceholder]
host = https://jsonplaceholder.typicode.com
@content-type = application/json

[localhost]
host = http://localhost:3000
@content-type = application/json

# Add your own profiles here:
# [myapi]
# host = https://api.example.com
# @authorization = Bearer your-token-here
# @content-type = application/json
EOF
    echo "Created initial profiles configuration: $PROFILES_FILE"
    echo ""
    echo "Example usage:"
    echo "  webly -p httpbin GET /get"
    echo "  webly -p jsonplaceholder GET /posts/1"
    echo ""
    echo "Edit $PROFILES_FILE to add your own API profiles."
else
    echo "Profiles file already exists: $PROFILES_FILE"
fi
